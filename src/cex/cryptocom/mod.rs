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
use std::collections::BTreeMap;
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
        symbol: &str,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        let cryptocom_symbol = format_symbol_for_exchange_ws(symbol, &CexExchange::Cryptocom)?;
        let channel = format!("book.{}.10", cryptocom_symbol);

        let (mut ws_stream, _) = tokio_tungstenite::connect_async(CRYPTOCOM_WS_MARKET)
            .await
            .map_err(|e| {
                MarketScannerError::ApiError(format!("Crypto.com WebSocket connect: {}", e))
            })?;

        let subscribe_msg = serde_json::json!({
            "id": 1,
            "method": "subscribe",
            "params": {
                "channels": [channel],
                "book_subscription_type": "SNAPSHOT_AND_UPDATE",
                "book_update_frequency": 100
            }
        });
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                subscribe_msg.to_string(),
            ))
            .await
            .map_err(|e| {
                MarketScannerError::ApiError(format!("Crypto.com WebSocket send: {}", e))
            })?;

        let (_write, mut read) = ws_stream.split();
        let (tx, rx) = mpsc::channel(64);
        let symbol_std = standard_symbol_for_cex_ws_response(symbol, &CexExchange::Cryptocom);

        tokio::spawn(async move {
            // Orderbook state: price (as rust_decimal for Ord) -> qty. Bids desc, asks asc.
            let mut bids: BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal> = BTreeMap::new();
            let mut asks: BTreeMap<rust_decimal::Decimal, rust_decimal::Decimal> = BTreeMap::new();

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

                let (data_bids, data_asks) = if channel == Some("book.update") {
                    // Delta: result.data[0].update.bids/asks or params.data.update
                    let item = value
                        .get("result")
                        .and_then(|r| r.get("data"))
                        .and_then(|d| d.as_array())
                        .and_then(|a| a.first())
                        .or_else(|| value.get("params").and_then(|p| p.get("data")));
                    let upd = item.and_then(|i| i.get("update"));
                    let b = upd.and_then(|u| u.get("bids"));
                    let a = upd.and_then(|u| u.get("asks"));
                    (b, a)
                } else {
                    // Full snapshot: replace book
                    let item = value.get("params").and_then(|p| p.get("data")).or_else(|| {
                        value
                            .get("result")
                            .and_then(|r| r.get("data"))
                            .and_then(|d| d.as_array())
                            .and_then(|a| a.first())
                    });
                    (
                        item.and_then(|i| i.get("bids")),
                        item.and_then(|i| i.get("asks")),
                    )
                };

                if channel == Some("book.update") {
                    apply_levels(&mut bids, data_bids);
                    apply_levels(&mut asks, data_asks);
                } else {
                    bids.clear();
                    asks.clear();
                    apply_levels(&mut bids, data_bids);
                    apply_levels(&mut asks, data_asks);
                }

                let Some((bid, ask, bid_qty, ask_qty)) = best_bid_ask(&bids, &asks) else {
                    continue;
                };

                let price = CexPrice {
                    symbol: symbol_std.clone(),
                    mid_price: find_mid_price(bid, ask),
                    bid_price: bid,
                    ask_price: ask,
                    bid_qty,
                    ask_qty,
                    timestamp: get_timestamp_millis(),
                    exchange: Exchange::Cex(CexExchange::Cryptocom),
                };
                if tx.send(price).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
