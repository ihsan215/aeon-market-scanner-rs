mod types;

use crate::cex::bybit::types::{BybitOrderbookWsMessage, BybitTickerData};
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, parse_f64, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

const BYBIT_API_BASE: &str = "https://api.bybit.com/v5";
const BYBIT_WS_SPOT: &str = "wss://stream.bybit.com/v5/public/spot";

create_exchange!(Bybit);

#[async_trait]
impl ExchangeTrait for Bybit {
    fn api_base(&self) -> &str {
        BYBIT_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bybit"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bybit market/time endpoint - test connectivity to the REST API
        let endpoint = "market/time";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Bybit {
    fn supports_websocket(&self) -> bool {
        true
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }
        // Format symbol for Bybit
        let bybit_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bybit)?;
        let endpoint = format!("market/tickers?category=spot&symbol={}", bybit_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success (Bybit uses camelCase in JSON)
        let ret_code = response["retCode"].as_i64().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API response missing retCode".to_string())
        })?;

        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Bybit API error: {} - {}",
                ret_code, ret_msg
            )));
        }

        // Parse the result.list array
        let list = response["result"]["list"].as_array().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API returned invalid data format".to_string())
        })?;

        let ticker_value = list.first().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API returned empty data".to_string())
        })?;

        // Deserialize the ticker data
        let ticker: BybitTickerData =
            serde_json::from_value(ticker_value.clone()).map_err(|e| {
                MarketScannerError::ApiError(format!("Failed to parse Bybit ticker data: {}", e))
            })?;

        let bid = parse_f64(&ticker.bid1_price, "bid price")?;
        let ask = parse_f64(&ticker.ask1_price, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid1_size, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask1_size, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol to standard format
        let standard_symbol = normalize_symbol(&ticker.symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bybit),
        })
    }

    /// Stream price via WebSocket (orderbook.1 spot). Connection stays open; prices sent over the channel.
    async fn stream_price_websocket(
        &self,
        symbols: &[&str],
        reconnect: bool,
        max_attempts: Option<u32>,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbols.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "At least one symbol required".to_string(),
            ));
        }

        let topics: Vec<String> = symbols
            .iter()
            .map(|s| {
                let sym = format_symbol_for_exchange_ws(s, &CexExchange::Bybit)?;
                Ok(format!("orderbook.1.{}", sym))
            })
            .collect::<Result<Vec<_>, MarketScannerError>>()?;

        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(BYBIT_WS_SPOT).await
                {
                    Ok(v) => v,
                    Err(_) => {
                        if !reconnect || tx.is_closed() {
                            break;
                        }
                        attempts = attempts.saturating_add(1);
                        if let Some(max) = max_attempts {
                            if attempts >= max {
                                break;
                            }
                        }
                        tokio::time::sleep(backoff).await;
                        backoff = std::cmp::min(max_backoff, backoff.saturating_mul(2));
                        continue;
                    }
                };

                backoff = std::time::Duration::from_secs(1);
                attempts = 0;

                let subscribe_msg = serde_json::json!({
                    "op": "subscribe",
                    "args": topics
                });
                if ws_stream
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        subscribe_msg.to_string(),
                    ))
                    .await
                    .is_err()
                {
                    if !reconnect || tx.is_closed() {
                        break;
                    }
                    attempts = attempts.saturating_add(1);
                    if let Some(max) = max_attempts {
                        if attempts >= max {
                            break;
                        }
                    }
                    continue;
                }

                let (_write, mut read) = ws_stream.split();

                while let Some(Ok(msg)) = read.next().await {
                    let text = match msg.into_text() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let parsed: BybitOrderbookWsMessage = match serde_json::from_str(&text) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    if parsed.msg_type != "snapshot" {
                        continue;
                    }
                    let data = &parsed.data;
                    let symbol_std =
                        standard_symbol_for_cex_ws_response(&data.symbol, &CexExchange::Bybit);
                    let (bid_price, bid_qty) = match data.bids.first() {
                        Some([p, q]) => {
                            let bp = match parse_f64(p, "bid price") {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                            let bq = parse_f64(q, "bid size").unwrap_or(0.0);
                            (bp, bq)
                        }
                        _ => continue,
                    };
                    let (ask_price, ask_qty) = match data.asks.first() {
                        Some([p, q]) => {
                            let ap = match parse_f64(p, "ask price") {
                                Ok(v) => v,
                                Err(_) => continue,
                            };
                            let aq = parse_f64(q, "ask size").unwrap_or(0.0);
                            (ap, aq)
                        }
                        _ => continue,
                    };
                    if bid_price <= 0.0 || ask_price <= 0.0 {
                        continue;
                    }
                    let price = CexPrice {
                        symbol: symbol_std.clone(),
                        mid_price: find_mid_price(bid_price, ask_price),
                        bid_price,
                        ask_price,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Bybit),
                    };
                    if tx.send(price).await.is_err() {
                        return;
                    }
                }

                if !reconnect || tx.is_closed() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
