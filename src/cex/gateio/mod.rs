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
        symbols: &[&str],
        reconnect_attempts: u32,
        reconnect_delay_ms: u64,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbols.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "At least one symbol required".to_string(),
            ));
        }

        let gateio_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Gateio))
            .collect::<Result<Vec<_>, _>>()?;

        let subscribe_msg = depth_subscribe_message(&gateio_symbols);

        let (tx, rx) = mpsc::channel(64);
        let delay = std::time::Duration::from_millis(if reconnect_delay_ms == 0 {
            1000
        } else {
            reconnect_delay_ms
        });

        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(GATEIO_WS_URL).await
                {
                    Ok(v) => v,
                    Err(_) => {
                        if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts
                        {
                            break;
                        }
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                };

                if ws_stream
                    .send(tokio_tungstenite::tungstenite::Message::Text(
                        subscribe_msg.to_string(),
                    ))
                    .await
                    .is_err()
                {
                    if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }

                let (_write, mut read) = ws_stream.split();

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
                    let market = params[2].as_str().unwrap_or("");
                    let symbol_std =
                        standard_symbol_for_cex_ws_response(market, &CexExchange::Gateio);
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
                        symbol: symbol_std,
                        mid_price: find_mid_price(bid, ask),
                        bid_price: bid,
                        ask_price: ask,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Gateio),
                    };
                    if tx.send(price).await.is_err() {
                        return;
                    }
                }

                if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                    break;
                }
                tokio::time::sleep(delay).await;
            }
        });

        Ok(rx)
    }
}

/// `depth.subscribe` request for several markets: `[[market, limit, interval], ...]`.
///
/// `interval` is the price-merge precision. It must be `"0"` (no merging): any other value
/// (e.g. `"0.01"`) rounds the book to that step, which corrupts prices of low-priced assets
/// (DOGE at ~0.067 would be reported as bid 0.06 / ask 0.07).
pub(crate) fn depth_subscribe_message(markets: &[String]) -> serde_json::Value {
    let params: Vec<serde_json::Value> = markets
        .iter()
        .map(|m| serde_json::json!([m, 10, "0"]))
        .collect();
    serde_json::json!({
        "id": 1,
        "method": "depth.subscribe",
        "params": params
    })
}

#[cfg(test)]
mod tests {
    use super::depth_subscribe_message;

    #[test]
    fn depth_subscribe_uses_unmerged_precision() {
        let msg = depth_subscribe_message(&["BTC_USDT".to_string(), "DOGE_USDT".to_string()]);
        assert_eq!(msg["method"], "depth.subscribe");
        let params = msg["params"].as_array().unwrap();
        assert_eq!(params.len(), 2);
        for p in params {
            assert_eq!(p[1], 10, "depth limit");
            assert_eq!(p[2], "0", "interval must be \"0\" (no price merging)");
        }
        assert_eq!(params[1][0], "DOGE_USDT");
    }
}
