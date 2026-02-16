mod types;

use crate::cex::kraken::types::KrakenDepthResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::collections::{BTreeMap, HashMap};
use tokio::sync::mpsc;

const KRAKEN_API_BASE: &str = "https://api.kraken.com/0/public";
const KRAKEN_WS_URL: &str = "wss://ws.kraken.com/v2";

create_exchange!(Kraken);

#[async_trait]
impl ExchangeTrait for Kraken {
    fn api_base(&self) -> &str {
        KRAKEN_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Kraken"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Kraken time endpoint - test connectivity to the REST API
        let endpoint = "Time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Kraken returns {"error": [], "result": {"unixtime": ..., "rfc1123": ...}}
        let error = response["error"].as_array();
        if let Some(errors) = error {
            if errors.is_empty() && response["result"].is_object() {
                Ok(())
            } else {
                Err(MarketScannerError::HealthCheckFailed)
            }
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for Kraken {
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

        // Format symbol for Kraken (BTC -> XBT conversion)
        let kraken_symbol = format_symbol_for_exchange(symbol, &CexExchange::Kraken)?;

        // Using Depth endpoint with count=1 for best bid/ask only
        let endpoint = format!("Depth?pair={}&count=1", kraken_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned errors
        let errors = response["error"].as_array().ok_or_else(|| {
            MarketScannerError::ApiError("Kraken API response missing error field".to_string())
        })?;

        if !errors.is_empty() {
            let error_msg = errors
                .iter()
                .filter_map(|e| e.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(MarketScannerError::ApiError(format!(
                "Kraken API error: {}",
                error_msg
            )));
        }

        // Deserialize response to KrakenDepthResponse
        let depth_response: KrakenDepthResponse =
            serde_json::from_value(response).map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Kraken API error: failed to parse depth response: {}",
                    e
                ))
            })?;

        // Get the first (and only) pair data from result
        let pair_data = depth_response.result.values().next().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no data found for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid (first element in bids array: [price, quantity, timestamp])
        let bid_entry = pair_data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity, timestamp])
        let ask_entry = pair_data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        // Parse bid entry: [price, quantity, timestamp]
        let bid_price_str = bid_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid bid price format for symbol: {}",
                symbol
            ))
        })?;

        let bid_qty_str = bid_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid bid quantity format for symbol: {}",
                symbol
            ))
        })?;

        // Parse ask entry: [price, quantity, timestamp]
        let ask_price_str = ask_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid ask price format for symbol: {}",
                symbol
            ))
        })?;

        let ask_qty_str = ask_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid ask quantity format for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(bid_price_str, "bid price")?;
        let ask = parse_f64(ask_price_str, "ask price")?;
        let bid_qty = parse_f64(bid_qty_str, "bid quantity")?;
        let ask_qty = parse_f64(ask_qty_str, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format (XBT -> BTC conversion)
        let standard_symbol = crate::common::normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Kraken),
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

        let kraken_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Kraken))
            .collect::<Result<Vec<_>, _>>()?;

        let subscribe_msg = serde_json::json!({
            "method": "subscribe",
            "params": {
                "channel": "book",
                "symbol": kraken_symbols,
                "depth": 10
            }
        });
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            type BookMap = BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>;
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            fn apply_kraken_levels(
                map: &mut BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>,
                arr: Option<&serde_json::Value>,
            ) {
                let arr = match arr.and_then(|a| a.as_array()) {
                    Some(a) => a,
                    None => return,
                };
                for level in arr {
                    let obj = match level.as_object() {
                        Some(o) => o,
                        None => continue,
                    };
                    let price_f = obj.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let qty_f = obj.get("qty").and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let price = rust_decimal::Decimal::from_f64_retain(price_f)
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    let qty = rust_decimal::Decimal::from_f64_retain(qty_f)
                        .unwrap_or(rust_decimal::Decimal::ZERO);
                    if qty.is_zero() {
                        map.remove(&price);
                    } else {
                        map.insert(price, qty);
                    }
                }
            }

            fn best_bid_ask(
                bids: &BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>,
                asks: &BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>,
            ) -> Option<(f64, f64, f64, f64)> {
                let (bid_price, bid_qty) = bids.iter().rev().next()?;
                let (ask_price, ask_qty) = asks.iter().next()?;
                let bid = bid_price.to_string().parse::<f64>().ok()?;
                let ask = ask_price.to_string().parse::<f64>().ok()?;
                let bq = bid_qty.to_string().parse::<f64>().ok()?;
                let aq = ask_qty.to_string().parse::<f64>().ok()?;
                if bid <= 0.0 || ask <= 0.0 {
                    return None;
                }
                Some((bid, ask, bq, aq))
            }

            loop {
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(KRAKEN_WS_URL).await
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

                let (mut write, mut read) = ws_stream.split();
                let mut books: HashMap<String, (BookMap, BookMap)> = HashMap::new();

                while let Some(Ok(msg)) = read.next().await {
                    let text = match msg.into_text() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };

                    // Server ping: respond with pong to keep connection alive
                    if value.get("method").and_then(|m| m.as_str()) == Some("ping") {
                        let req_id = value.get("req_id").cloned();
                        let pong = match req_id {
                            Some(id) => serde_json::json!({ "method": "pong", "req_id": id }),
                            None => serde_json::json!({ "method": "pong" }),
                        };
                        let _ = write
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                pong.to_string(),
                            ))
                            .await;
                        continue;
                    }

                    // Heartbeat: {"channel":"heartbeat"} - no response needed, skip
                    if value.get("channel").and_then(|c| c.as_str()) == Some("heartbeat") {
                        continue;
                    }

                    // Subscribe ack: {"method":"subscribe","result":{...},"success":true}
                    if value.get("method").and_then(|m| m.as_str()) == Some("subscribe") {
                        continue;
                    }

                    // Book snapshot/update: channel=book, type=snapshot|update, data=[{symbol, bids, asks}, ...]
                    if value.get("channel").and_then(|c| c.as_str()) != Some("book") {
                        continue;
                    }

                    let data_arr = match value.get("data").and_then(|d| d.as_array()) {
                        Some(d) if !d.is_empty() => d,
                        _ => continue,
                    };

                    let msg_type = value.get("type").and_then(|t| t.as_str());

                    for data in data_arr {
                        let kraken_sym = match data.get("symbol").and_then(|s| s.as_str()) {
                            Some(s) => s,
                            None => continue,
                        };
                        let symbol_std =
                            standard_symbol_for_cex_ws_response(kraken_sym, &CexExchange::Kraken);
                        let (bids, asks) = books
                            .entry(symbol_std.clone())
                            .or_insert_with(|| (BTreeMap::new(), BTreeMap::new()));
                        if msg_type == Some("snapshot") {
                            bids.clear();
                            asks.clear();
                        }
                        apply_kraken_levels(bids, data.get("bids"));
                        apply_kraken_levels(asks, data.get("asks"));

                        let (bid, ask, bid_qty, ask_qty) = match best_bid_ask(bids, asks) {
                            Some(b) => b,
                            None => continue,
                        };

                        let price = CexPrice {
                            symbol: symbol_std.clone(),
                            mid_price: find_mid_price(bid, ask),
                            bid_price: bid,
                            ask_price: ask,
                            bid_qty,
                            ask_qty,
                            timestamp: get_timestamp_millis(),
                            exchange: Exchange::Cex(CexExchange::Kraken),
                        };
                        if tx.send(price).await.is_err() {
                            return;
                        }
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
