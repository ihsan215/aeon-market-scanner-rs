mod types;

use crate::cex::coinbase::types::{CoinbaseOrderBookResponse, CoinbaseTickerWs};
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

const COINBASE_API_BASE: &str = "https://api.exchange.coinbase.com";
const COINBASE_WS_FEED: &str = "wss://ws-feed.exchange.coinbase.com";

create_exchange!(Coinbase);

#[async_trait]
impl ExchangeTrait for Coinbase {
    fn api_base(&self) -> &str {
        COINBASE_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Coinbase"
    }

    // Override get method to add User-Agent header
    async fn get<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
    ) -> Result<T, MarketScannerError> {
        let url = format!("{}/{}", self.api_base(), endpoint);
        let response = self
            .client()
            .get(&url)
            .header("User-Agent", "aeon-market-scanner-rs")
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // For 404, include the endpoint in error message
            if status == 404 {
                return Err(MarketScannerError::ApiError(format!(
                    "{} API error: {} - {} (endpoint: {})",
                    self.exchange_name(),
                    status,
                    error_text,
                    endpoint
                )));
            }
            return Err(MarketScannerError::ApiError(format!(
                "{} API error: {} - {}",
                self.exchange_name(),
                status,
                error_text
            )));
        }

        Ok(response.json().await?)
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Coinbase time endpoint - test connectivity to the REST API
        let endpoint = "time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Coinbase returns {"iso": "...", "epoch": ...}
        if response["iso"].is_string() {
            Ok(())
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for Coinbase {
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

        // Format symbol for Coinbase
        let coinbase_symbol = format_symbol_for_exchange(symbol, &CexExchange::Coinbase)?;

        // Using orderbook endpoint with level=1 for best bid/ask only
        let endpoint = format!("products/{}/book?level=1", coinbase_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response has error
        if let Some(message) = response.get("message") {
            if message.as_str() == Some("NotFound") {
                return Err(MarketScannerError::ApiError(format!(
                    "Coinbase API error: symbol {} not found (tried endpoint: products/{}/book?level=1)",
                    symbol, coinbase_symbol
                )));
            }
        }

        // Deserialize response directly to CoinbaseOrderBookResponse
        let orderbook_response: CoinbaseOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Coinbase API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get best bid (first element in bids array: [price, quantity, order_count])
        let bid_entry = orderbook_response.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity, order_count])
        let ask_entry = orderbook_response.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        // Parse bid entry: [price, quantity, order_count]
        let bid_price_str = bid_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid bid price format for symbol: {}",
                symbol
            ))
        })?;

        let bid_qty_str = bid_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid bid quantity format for symbol: {}",
                symbol
            ))
        })?;

        // Parse ask entry: [price, quantity, order_count]
        let ask_price_str = ask_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid ask price format for symbol: {}",
                symbol
            ))
        })?;

        let ask_qty_str = ask_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid ask quantity format for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(bid_price_str, "bid price")?;
        let ask = parse_f64(ask_price_str, "ask price")?;
        let bid_qty = parse_f64(bid_qty_str, "bid quantity")?;
        let ask_qty = parse_f64(ask_qty_str, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert Coinbase symbol format (BTC-USDT) back to standard (BTCUSDT)
        let standard_symbol = coinbase_symbol.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Coinbase),
        })
    }

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

        let coinbase_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Coinbase))
            .collect::<Result<Vec<_>, _>>()?;

        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(COINBASE_WS_FEED).await {
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
                    "type": "subscribe",
                    "product_ids": coinbase_symbols,
                    "channels": ["ticker"]
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
                    let ticker: CoinbaseTickerWs = match serde_json::from_str(&text) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    if ticker.msg_type != "ticker" {
                        continue;
                    }
                    let bid = match parse_f64(&ticker.best_bid, "bid") {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let ask = match parse_f64(&ticker.best_ask, "ask") {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let bid_qty = parse_f64(&ticker.best_bid_size, "bid_size").unwrap_or(0.0);
                    let ask_qty = parse_f64(&ticker.best_ask_size, "ask_size").unwrap_or(0.0);
                    if bid <= 0.0 || ask <= 0.0 {
                        continue;
                    }
                    let symbol_std = standard_symbol_for_cex_ws_response(
                        &ticker.product_id,
                        &CexExchange::Coinbase,
                    );
                    let price = CexPrice {
                        symbol: symbol_std,
                        mid_price: find_mid_price(bid, ask),
                        bid_price: bid,
                        ask_price: ask,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Coinbase),
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
