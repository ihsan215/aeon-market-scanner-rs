mod types;

use crate::cex::okx::types::OkxTickerResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const OKX_API_BASE: &str = "https://www.okx.com/api/v5";
const OKX_WS_URL: &str = "wss://ws.okx.com:8443/ws/v5/public";

create_exchange!(OKX);

#[async_trait]
impl ExchangeTrait for OKX {
    fn api_base(&self) -> &str {
        OKX_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "OKX"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // OKX public/time endpoint - returns server time
        let endpoint = "public/time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // OKX returns {"code":"0", "data":[...], "msg":""} for success
        if let Some(code) = response["code"].as_str() {
            if code == "0" {
                Ok(())
            } else {
                let msg = response["msg"].as_str().unwrap_or("Unknown error");
                Err(MarketScannerError::ApiError(format!(
                    "OKX health check failed: {} - {}",
                    code, msg
                )))
            }
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for OKX {
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
        // Format symbol for OKX
        let okx_symbol = format_symbol_for_exchange(symbol, &CexExchange::OKX)?;
        let endpoint = format!("market/ticker?instId={}", okx_symbol);

        let response: OkxTickerResponse = self.get(&endpoint).await?;

        // Check if API returned success
        if response.code != "0" {
            return Err(MarketScannerError::ApiError(format!(
                "OKX API error: {} - {}",
                response.code, response.msg
            )));
        }

        // Get first ticker data
        let ticker = response.data.first().ok_or_else(|| {
            MarketScannerError::ApiError("OKX API returned empty data".to_string())
        })?;

        let bid = parse_f64(&ticker.bid_px, "bid price")?;
        let ask = parse_f64(&ticker.ask_px, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid_sz, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_sz, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

        // Convert OKX symbol format (BTC-USDT) to standard (BTCUSDT)
        let standard_symbol = ticker.inst_id.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::OKX),
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

        let okx_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::OKX))
            .collect::<Result<Vec<_>, _>>()?;

        // Use orderbook top-of-book via books5: bids/asks arrays.
        // Subscribe: {"op":"subscribe","args":[{"channel":"books5","instId":"BTC-USDT"}, ...]}
        let args: Vec<serde_json::Value> = okx_symbols
            .iter()
            .map(|inst_id| serde_json::json!({"channel": "books5", "instId": inst_id}))
            .collect();
        let subscribe_msg = serde_json::json!({ "op": "subscribe", "args": args });

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
                let (ws_stream, _) = match tokio_tungstenite::connect_async(OKX_WS_URL).await {
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

                let (mut write, mut read) = ws_stream.split();

                if write
                    .send(WsMessage::Text(subscribe_msg.to_string()))
                    .await
                    .is_err()
                {
                    if tx.is_closed() || reconnect_attempts == 0 || attempt > reconnect_attempts {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }

                let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(20));
                ping_interval.tick().await;

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            // Prefer websocket ping frame; OKX also supports text ping/pong.
                            if write.send(WsMessage::Ping(Vec::new())).await.is_err() {
                                break;
                            }
                        }
                        msg = read.next() => {
                            let msg = match msg {
                                Some(Ok(m)) => m,
                                _ => break,
                            };

                            match msg {
                                WsMessage::Ping(payload) => {
                                    let _ = write.send(WsMessage::Pong(payload)).await;
                                }
                                WsMessage::Pong(_) => {}
                                WsMessage::Text(t) => {
                                    // OKX may also send raw "pong"
                                    if t == "pong" || t == "ping" {
                                        if t == "ping" {
                                            let _ = write.send(WsMessage::Text("pong".to_string())).await;
                                        }
                                        continue;
                                    }

                                    let v: serde_json::Value = match serde_json::from_str(&t) {
                                        Ok(v) => v,
                                        Err(_) => continue,
                                    };

                                    // events: {"event":"subscribe",...} / {"event":"error",...}
                                    if v.get("event").and_then(|e| e.as_str()).is_some() {
                                        continue;
                                    }

                                    let data = match v.get("data").and_then(|d| d.as_array()) {
                                        Some(d) if !d.is_empty() => d,
                                        _ => continue,
                                    };

                                    // arg.instId fallback for some payloads
                                    let arg_inst = v.get("arg")
                                        .and_then(|a| a.get("instId"))
                                        .and_then(|s| s.as_str());

                                    for item in data {
                                        if let Some(price) = parse_okx_books5(item, arg_inst) {
                                            if tx.send(price).await.is_err() {
                                                return;
                                            }
                                        }
                                    }
                                }
                                WsMessage::Binary(_) => {}
                                WsMessage::Close(_) => break,
                                _ => {}
                            }
                        }
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

fn json_to_f64(v: &serde_json::Value) -> Option<f64> {
    if let Some(s) = v.as_str() {
        parse_f64(s, "value").ok()
    } else if let Some(n) = v.as_f64() {
        Some(n)
    } else if let Some(n) = v.as_u64() {
        Some(n as f64)
    } else if let Some(n) = v.as_i64() {
        Some(n as f64)
    } else {
        None
    }
}

fn parse_okx_books5(item: &serde_json::Value, arg_inst: Option<&str>) -> Option<CexPrice> {
    let inst_id = item.get("instId").and_then(|s| s.as_str()).or(arg_inst)?;

    let bids = item.get("bids")?.as_array()?;
    let asks = item.get("asks")?.as_array()?;
    let bid_entry = bids.first()?.as_array()?;
    let ask_entry = asks.first()?.as_array()?;
    if bid_entry.len() < 2 || ask_entry.len() < 2 {
        return None;
    }

    let bid = json_to_f64(&bid_entry[0])?;
    let bid_qty = json_to_f64(&bid_entry[1]).unwrap_or(0.0);
    let ask = json_to_f64(&ask_entry[0])?;
    let ask_qty = json_to_f64(&ask_entry[1]).unwrap_or(0.0);
    if bid <= 0.0 || ask <= 0.0 {
        return None;
    }

    let symbol = standard_symbol_for_cex_ws_response(inst_id, &CexExchange::OKX);

    Some(CexPrice {
        symbol,
        mid_price: find_mid_price(bid, ask),
        bid_price: bid,
        ask_price: ask,
        bid_qty,
        ask_qty,
        timestamp: get_timestamp_millis(),
        exchange: Exchange::Cex(CexExchange::OKX),
    })
}
