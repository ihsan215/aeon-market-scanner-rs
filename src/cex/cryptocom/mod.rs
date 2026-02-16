mod types;

use crate::cex::cryptocom::types::CryptocomOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, parse_f64, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use std::collections::{BTreeMap, HashMap};
use tokio::sync::mpsc;

const CRYPTOCOM_API_BASE: &str = "https://api.crypto.com/v2/public";
const CRYPTOCOM_WS_MARKET: &str = "wss://stream.crypto.com/v2/market";

create_exchange!(Cryptocom);

#[async_trait]
impl ExchangeTrait for Cryptocom {
    fn api_base(&self) -> &str {
        CRYPTOCOM_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Crypto.com"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Crypto.com Exchange book endpoint - test connectivity with BTC_USDT
        // Time endpoint returns BAD_REQUEST, so we use get-book instead
        // Note: api_base already includes /public, so we don't need to prefix with "public/"
        let endpoint = "get-book?instrument_name=BTC_USDT&depth=1";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Check if response indicates successful connection
        // Crypto.com returns {"code": 0, "result": {...}}
        if let Some(code) = response.get("code") {
            if code.as_i64() == Some(0) {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }
}

#[async_trait]
impl CEXTrait for Cryptocom {
    fn supports_websocket(&self) -> bool {
        true
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Crypto.com Exchange
        let cryptocom_symbol = format_symbol_for_exchange(symbol, &CexExchange::Cryptocom)?;

        // Get orderbook
        // Note: api_base already includes /public, so we don't need to prefix with "public/"
        let endpoint = format!("get-book?instrument_name={}&depth=1", cryptocom_symbol);

        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check for errors in response
        if let Some(code) = response.get("code") {
            if code.as_i64() != Some(0) {
                if let Some(msg) = response.get("message") {
                    return Err(MarketScannerError::ApiError(format!(
                        "Crypto.com API error: {} - {}",
                        code, msg
                    )));
                }
            }
        }

        // Parse orderbook response
        let orderbook_response: CryptocomOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Crypto.com API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get first data entry (should be for the requested symbol)
        let orderbook_data = orderbook_response.result.data.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no orderbook data found for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid and ask
        let bid_entry = orderbook_data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        let ask_entry = orderbook_data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);
        let standard_symbol = normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Cryptocom),
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

        let channels: Vec<String> = symbols
            .iter()
            .map(|s| {
                let sym = format_symbol_for_exchange_ws(s, &CexExchange::Cryptocom)?;
                Ok(format!("book.{}.10", sym))
            })
            .collect::<Result<Vec<_>, MarketScannerError>>()?;

        let subscribe_msg = serde_json::json!({
            "id": 1,
            "method": "subscribe",
            "params": {
                "channels": channels,
                "book_subscription_type": "SNAPSHOT_AND_UPDATE",
                "book_update_frequency": 100
            }
        });
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            type BookMap = BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>;
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            fn apply_levels(
                map: &mut BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal>,
                arr: Option<&serde_json::Value>,
            ) {
                let arr = match arr.and_then(|a| a.as_array()) {
                    Some(a) => a,
                    None => return,
                };
                for level in arr {
                    let level = match level.as_array().filter(|l| l.len() >= 2) {
                        Some(l) => l,
                        None => continue,
                    };
                    let price_str = level[0].as_str().unwrap_or("");
                    let qty_str = level[1].as_str().unwrap_or("");
                    let price: rust_decimal::Decimal = price_str.parse().unwrap_or_default();
                    let qty: rust_decimal::Decimal = qty_str.parse().unwrap_or_default();
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
                let (mut ws_stream, _) =
                    match tokio_tungstenite::connect_async(CRYPTOCOM_WS_MARKET).await {
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
                    continue;
                }

                let (_write, mut read) = ws_stream.split();
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
                    // Skip subscribe ack (has method=subscribe but no book data)
                    if value.get("method").and_then(|m| m.as_str()) == Some("subscribe") {
                        let has_data = value.get("params").and_then(|p| p.get("data")).is_some()
                            || value.get("result").and_then(|r| r.get("data")).is_some();
                        if !has_data {
                            continue;
                        }
                    }

                    let channel = value
                        .get("params")
                        .and_then(|p| p.get("channel"))
                        .and_then(|c| c.as_str())
                        .or_else(|| {
                            value
                                .get("result")
                                .and_then(|r| r.get("channel"))
                                .and_then(|c| c.as_str())
                        });

                    let result_obj = value.get("result");
                    let params_obj = value.get("params");
                    let item = result_obj
                        .and_then(|r| r.get("data"))
                        .and_then(|d| d.as_array())
                        .and_then(|a| a.first())
                        .or_else(|| params_obj.and_then(|p| p.get("data")));
                    let item = match item {
                        Some(i) => i,
                        None => continue,
                    };

                    // Get symbol: result.instrument_name, result.subscription "book.BTC_USDT.10", channel "book.BTC_USDT.10", item.instrument_name
                    // Note: channel is "book.update" for deltas - do NOT parse channel for symbol in that case
                    let cryptocom_sym = result_obj
                        .and_then(|r| r.get("instrument_name"))
                        .and_then(|v| v.as_str())
                        .or_else(|| {
                            result_obj
                                .and_then(|r| r.get("subscription"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.strip_prefix("book."))
                                .and_then(|s| s.split('.').next())
                        })
                        .or_else(|| {
                            params_obj
                                .and_then(|p| p.get("subscription"))
                                .and_then(|v| v.as_str())
                                .and_then(|s| s.strip_prefix("book."))
                                .and_then(|s| s.split('.').next())
                        })
                        .or_else(|| {
                            // Only parse channel if it looks like "book.X.10" (not "book.update")
                            channel
                                .filter(|c| !c.contains("update"))
                                .and_then(|c| c.strip_prefix("book."))
                                .and_then(|s| s.split('.').next())
                        })
                        .or_else(|| item.get("instrument_name").and_then(|v| v.as_str()));
                    let symbol_std = match cryptocom_sym {
                        Some(s) => standard_symbol_for_cex_ws_response(s, &CexExchange::Cryptocom),
                        None => continue,
                    };

                    let (data_bids, data_asks) = if channel == Some("book.update") {
                        let upd = item.get("update");
                        (
                            upd.and_then(|u| u.get("bids")),
                            upd.and_then(|u| u.get("asks")),
                        )
                    } else {
                        (item.get("bids"), item.get("asks"))
                    };

                    let (bids, asks) = books
                        .entry(symbol_std.clone())
                        .or_insert_with(|| (BTreeMap::new(), BTreeMap::new()));
                    if channel == Some("book.update") {
                        apply_levels(bids, data_bids);
                        apply_levels(asks, data_asks);
                    } else {
                        bids.clear();
                        asks.clear();
                        apply_levels(bids, data_bids);
                        apply_levels(asks, data_asks);
                    }

                    let Some((bid, ask, bid_qty, ask_qty)) = best_bid_ask(bids, asks) else {
                        continue;
                    };

                    let price = CexPrice {
                        symbol: symbol_std,
                        mid_price: find_mid_price(bid, ask),
                        bid_price: bid,
                        ask_price: ask,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Cryptocom),
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
