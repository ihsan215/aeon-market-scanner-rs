mod types;

use crate::cex::bitfinex::types::BitfinexOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

const BITFINEX_API_BASE: &str = "https://api-pub.bitfinex.com/v2";
const BITFINEX_WS_URL: &str = "wss://api-pub.bitfinex.com/ws/2";

create_exchange!(Bitfinex);

#[async_trait]
impl ExchangeTrait for Bitfinex {
    fn api_base(&self) -> &str {
        BITFINEX_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bitfinex"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bitfinex platform status endpoint - test connectivity to the REST API
        let endpoint = "platform/status";
        let response: types::BitfinexPlatformStatus = self.get(endpoint).await?;

        // Bitfinex returns [1] for operational, [0] for maintenance
        if let Some(code) = response.first() {
            if *code == 1 {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }
}

#[async_trait]
impl CEXTrait for Bitfinex {
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

        // Format symbol for Bitfinex (tBTCUSD format)
        let bitfinex_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bitfinex)?;

        // Using orderbook endpoint with P0 precision and len=1 for best bid/ask only
        let endpoint = format!("book/{}/P0?len=1", bitfinex_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response is an error array (Bitfinex v2 returns errors as [error_code, "error_message"])
        if let Some(array) = response.as_array() {
            if array.len() == 2 {
                if let (Some(code), Some(msg)) = (
                    array.get(0).and_then(|v| v.as_i64()),
                    array.get(1).and_then(|v| v.as_str()),
                ) {
                    if code != 0 {
                        return Err(MarketScannerError::ApiError(format!(
                            "Bitfinex API error: {} - {}",
                            code, msg
                        )));
                    }
                }
            }
        }

        // Deserialize response to BitfinexOrderBookResponse
        // Bitfinex returns orderbook as array: [[price, count, amount], ...]
        let orderbook_response: BitfinexOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Separate bids (negative amount) and asks (positive amount)
        // Bitfinex: amount < 0 means bid (buy order), amount > 0 means ask (sell order)
        let mut bids: Vec<(f64, f64)> = Vec::new();
        let mut asks: Vec<(f64, f64)> = Vec::new();

        for entry in orderbook_response {
            let price = entry[0];
            let _count = entry[1] as i64;
            let amount = entry[2];

            if amount < 0.0 {
                // Bid (negative amount) - buyers want to buy at this price
                bids.push((price, amount.abs()));
            } else if amount > 0.0 {
                // Ask (positive amount) - sellers want to sell at this price
                asks.push((price, amount));
            }
        }

        // Get best bid (highest bid price - buyers want highest price they're willing to pay)
        let bid_entry = bids
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: no bid found for symbol: {}",
                    symbol
                ))
            })?;

        // Get best ask (lowest ask price - sellers want lowest price they're willing to accept)
        let ask_entry = asks
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: no ask found for symbol: {}",
                    symbol
                ))
            })?;

        let mut bid = bid_entry.0;
        let mut ask = ask_entry.0;
        let mut bid_qty = bid_entry.1;
        let mut ask_qty = ask_entry.1;

        // Ensure bid <= ask (if not, swap them as this shouldn't happen but Bitfinex might return them reversed)
        if bid > ask {
            std::mem::swap(&mut bid, &mut ask);
            std::mem::swap(&mut bid_qty, &mut ask_qty);
        }

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        // Bitfinex converts USDT to UST, so we need to convert back
        // But we should preserve what was actually used on the exchange
        // Since we converted BTCUSDT -> tBTCUST, we should return BTCUST in the response
        let standard_symbol = if symbol.to_uppercase().ends_with("USDT") {
            // Convert back: BTCUSDT -> BTCUST (what Bitfinex actually uses)
            let base = symbol
                .to_uppercase()
                .replace("-", "")
                .replace("_", "")
                .replace("USDT", "UST");
            base
        } else {
            normalize_symbol(symbol)
        };

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bitfinex),
        })
    }

    /// Connection stays open; incoming ticker updates are sent over the returned Receiver.
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

        let bitfinex_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Bitfinex))
            .collect::<Result<Vec<_>, _>>()?;

        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                let (mut ws_stream, _) =
                    match tokio_tungstenite::connect_async(BITFINEX_WS_URL).await {
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

                for bitfinex_symbol in &bitfinex_symbols {
                    let subscribe_msg = serde_json::json!({
                        "event": "subscribe",
                        "channel": "ticker",
                        "symbol": bitfinex_symbol
                    });
                    if ws_stream
                        .send(tokio_tungstenite::tungstenite::Message::Text(
                            subscribe_msg.to_string(),
                        ))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }

                let (_write, mut read) = ws_stream.split();
                let mut chan_to_symbol: std::collections::HashMap<u64, String> =
                    std::collections::HashMap::new();

                while let Some(Ok(msg)) = read.next().await {
                    let text = match msg.into_text() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if let (Some(ev), Some(chan_id), Some(sym)) = (
                        value.get("event").and_then(|e| e.as_str()),
                        value.get("chanId").and_then(|c| c.as_u64()),
                        value.get("symbol").and_then(|s| s.as_str()),
                    ) {
                        if ev == "subscribed" {
                            chan_to_symbol.insert(
                                chan_id,
                                standard_symbol_for_cex_ws_response(sym, &CexExchange::Bitfinex),
                            );
                        }
                        continue;
                    }
                    let arr = match value.as_array() {
                        Some(a) if a.len() >= 2 => a,
                        _ => continue,
                    };
                    let chan_id = match arr[0].as_u64() {
                        Some(id) => id,
                        None => continue,
                    };
                    let symbol_std = match chan_to_symbol.get(&chan_id) {
                        Some(s) => s.clone(),
                        None => continue,
                    };
                    let data = match arr[1].as_array() {
                        Some(d) if d.len() >= 4 => d,
                        _ => continue,
                    };
                    let bid = match data[0].as_f64() {
                        Some(b) if b > 0.0 => b,
                        _ => continue,
                    };
                    let bid_qty = data[1].as_f64().unwrap_or(0.0).abs();
                    let ask = match data[2].as_f64() {
                        Some(a) if a > 0.0 => a,
                        _ => continue,
                    };
                    let ask_qty = data[3].as_f64().unwrap_or(0.0).abs();
                    let price = CexPrice {
                        symbol: symbol_std,
                        mid_price: find_mid_price(bid, ask),
                        bid_price: bid,
                        ask_price: ask,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Bitfinex),
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
