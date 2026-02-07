mod types;

use crate::cex::gateio::types::GateioOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;

const GATEIO_API_BASE: &str = "https://api.gateio.ws/api/v4";
// WebSocket v3: wss://ws.gate.io/v3/ - method/params format (depth.subscribe)
const GATEIO_WS_URL: &str = "wss://ws.gate.io/v3/";

create_exchange!(Gateio);

#[async_trait]
impl ExchangeTrait for Gateio {
    fn api_base(&self) -> &str {
        GATEIO_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Gate.io"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Gate.io time endpoint - test connectivity to the REST API
        let endpoint = "spot/time";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Gateio {
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

        // Format symbol for Gate.io
        let gateio_symbol = format_symbol_for_exchange(symbol, &CexExchange::Gateio)?;

        // Get order book for bid/ask prices and quantities (limit=1 for best bid/ask only)
        let book_endpoint = format!("spot/order_book?currency_pair={}&limit=1", gateio_symbol);
        let order_book: GateioOrderBookResponse = self.get(&book_endpoint).await?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = order_book.bids.first().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("No bid found for symbol: {}", symbol))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = order_book.asks.first().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("No ask found for symbol: {}", symbol))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert Gate.io symbol format (BTC_USDT) back to standard (BTCUSDT)
        let standard_symbol = gateio_symbol.replace("_", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Gateio),
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

        let gateio_symbol = format_symbol_for_exchange_ws(symbol, &CexExchange::Gateio)?;

        let (mut ws_stream, _) = tokio_tungstenite::connect_async(GATEIO_WS_URL)
            .await
            .map_err(|e| {
                MarketScannerError::ApiError(format!("Gate.io WebSocket connect: {}", e))
            })?;

        // depth.subscribe: params [market, limit, interval]
        // limit: 1,5,10,20,30 | interval: "0","0.0001","0.001","0.01","0.1" etc.
        let subscribe_msg = serde_json::json!({
            "id": 1,
            "method": "depth.subscribe",
            "params": [gateio_symbol, 10, "0.01"]
        });

        ws_stream
            .send(tokio_tungstenite::tungstenite::Message::Text(
                subscribe_msg.to_string(),
            ))
            .await
            .map_err(|e| MarketScannerError::ApiError(format!("Gate.io WebSocket send: {}", e)))?;

        let (_write, mut read) = ws_stream.split();
        let (tx, rx) = mpsc::channel(64);
        let symbol_std = standard_symbol_for_cex_ws_response(symbol, &CexExchange::Gateio);

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
                // Skip subscribe ack: {"error":null,"result":{"status":"success"},"id":1}
                if value.get("id").is_some() && value.get("id").unwrap().is_number() {
                    let result = value.get("result");
                    if result
                        .and_then(|r| r.get("status"))
                        .and_then(|s| s.as_str())
                        == Some("success")
                    {
                        continue;
                    }
                    if value.get("error").is_some() {
                        continue;
                    }
                }
                // depth.update: params = [clean, depth, market]; depth has bids/asks
                if value.get("method").and_then(|m| m.as_str()) != Some("depth.update") {
                    continue;
                }
                let params = match value.get("params").and_then(|p| p.as_array()) {
                    Some(p) if p.len() >= 3 => p,
                    _ => continue,
                };
                let depth = match params[1].as_object() {
                    Some(d) => d,
                    None => continue,
                };
                let bids = depth.get("bids").and_then(|v| v.as_array());
                let asks = depth.get("asks").and_then(|v| v.as_array());
                let (bid_entry, ask_entry) = match (bids, asks) {
                    (Some(b), Some(a)) => {
                        let be = b.first().and_then(|x| x.as_array());
                        let ae = a.first().and_then(|x| x.as_array());
                        match (be, ae) {
                            (Some(be), Some(ae)) if be.len() >= 2 && ae.len() >= 2 => (be, ae),
                            _ => continue,
                        }
                    }
                    _ => continue,
                };
                let bid_str = bid_entry[0].as_str().unwrap_or("");
                let bid_qty_str = bid_entry[1].as_str().unwrap_or("0");
                let ask_str = ask_entry[0].as_str().unwrap_or("");
                let ask_qty_str = ask_entry[1].as_str().unwrap_or("0");
                let bid = match parse_f64(bid_str, "bid") {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let ask = match parse_f64(ask_str, "ask") {
                    Ok(v) => v,
                    Err(_) => continue,
                };
                let bid_qty = parse_f64(bid_qty_str, "bid_qty").unwrap_or(0.0);
                let ask_qty = parse_f64(ask_qty_str, "ask_qty").unwrap_or(0.0);
                if bid <= 0.0 || ask <= 0.0 {
                    continue;
                }
                let price = CexPrice {
                    symbol: symbol_std.clone(),
                    mid_price: find_mid_price(bid, ask),
                    bid_price: bid,
                    ask_price: ask,
                    bid_qty,
                    ask_qty,
                    timestamp: get_timestamp_millis(),
                    exchange: Exchange::Cex(CexExchange::Gateio),
                };
                if tx.send(price).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
