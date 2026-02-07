mod types;

use crate::cex::bitget::types::BitgetOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

const BITGET_API_BASE: &str = "https://api.bitget.com/api/v2";
const BITGET_WS_URL: &str = "wss://ws.bitget.com/v2/ws/public";

create_exchange!(Bitget);

#[async_trait]
impl ExchangeTrait for Bitget {
    fn api_base(&self) -> &str {
        BITGET_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bitget"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bitget public server time endpoint - test connectivity to the REST API
        let endpoint = "public/time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Check if API returned success (Bitget uses "00000" for success)
        let code = response["code"].as_str();
        if code == Some("00000") {
            Ok(())
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for Bitget {
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

        // Format symbol for Bitget
        let bitget_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bitget)?;
        // Using v2 API orderbook endpoint (limit=1 for best bid/ask only)
        let endpoint = format!("spot/market/orderbook?symbol={}&limit=1", bitget_symbol);

        // First get as JSON value to check code
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success (Bitget uses "00000" for success)
        let code = response["code"].as_str().ok_or_else(|| {
            MarketScannerError::ApiError("Bitget API response missing code".to_string())
        })?;

        if code != "00000" {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Bitget API error: {} - {}",
                code, msg
            )));
        }

        // Deserialize response to BitgetOrderBookResponse using type definitions
        let orderbook_response: BitgetOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Bitget API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get data object
        let data = orderbook_response.data.ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: returned null or invalid data for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        let standard_symbol = crate::common::normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bitget),
        })
    }

    /// Connection stays open; incoming ticker updates are sent over the returned Receiver.
    async fn stream_price_websocket(
        &self,
        symbol: &str,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        let bitget_symbol = format_symbol_for_exchange_ws(symbol, &CexExchange::Bitget)?;

        let (mut ws_stream, _) = tokio_tungstenite::connect_async(BITGET_WS_URL)
            .await
            .map_err(|e| {
                MarketScannerError::ApiError(format!("Bitget WebSocket connect: {}", e))
            })?;

        let subscribe_msg = serde_json::json!({
            "op": "subscribe",
            "args": [{
                "instType": "SPOT",
                "channel": "ticker",
                "instId": bitget_symbol
            }]
        });
        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                subscribe_msg.to_string(),
            ))
            .await
            .map_err(|e| MarketScannerError::ApiError(format!("Bitget WebSocket send: {}", e)))?;

        let (_write, mut read) = ws_stream.split();
        let (tx, rx) = mpsc::channel(64);
        let symbol_std = standard_symbol_for_cex_ws_response(symbol, &CexExchange::Bitget);

        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                let text = match msg.into_text() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let value: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                if value.get("event").is_some()
                    || value.get("op").and_then(|o| o.as_str()) == Some("subscribe")
                {
                    continue;
                }
                let data_arr = match value.get("data").and_then(|d| d.as_array()) {
                    Some(a) if !a.is_empty() => a,
                    _ => continue,
                };
                let item = &data_arr[0];
                let (b, bq, a, aq) = if item.is_object() {
                    let bid_pr = item
                        .get("bidPr")
                        .or(item.get("bidPx"))
                        .and_then(|v| v.as_str());
                    let ask_pr = item
                        .get("askPr")
                        .or(item.get("askPx"))
                        .and_then(|v| v.as_str());
                    let bid_sz = item.get("bidSz").and_then(|v| v.as_str());
                    let ask_sz = item.get("askSz").and_then(|v| v.as_str());
                    let bid_f = bid_pr.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let ask_f = ask_pr.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let bid_q = bid_sz.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    let ask_q = ask_sz.and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
                    (bid_f, bid_q, ask_f, ask_q)
                } else if let Some(arr) = item.as_array() {
                    if arr.len() >= 4 {
                        let parse = |i: usize| {
                            arr.get(i)
                                .and_then(|v| v.as_str().and_then(|s| s.parse::<f64>().ok()))
                                .unwrap_or(0.0)
                        };
                        (parse(2), 0.0, parse(3), 0.0)
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                if b <= 0.0 || a <= 0.0 {
                    continue;
                }
                let price = CexPrice {
                    symbol: symbol_std.clone(),
                    mid_price: find_mid_price(b, a),
                    bid_price: b,
                    ask_price: a,
                    bid_qty: bq,
                    ask_qty: aq,
                    timestamp: get_timestamp_millis(),
                    exchange: Exchange::Cex(CexExchange::Bitget),
                };
                if tx.send(price).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
