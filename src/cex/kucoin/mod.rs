mod types;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis, parse_f64,
    standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const KUCOIN_API_BASE: &str = "https://api.kucoin.com/api/v1";

create_exchange!(Kucoin);

#[async_trait]
impl ExchangeTrait for Kucoin {
    fn api_base(&self) -> &str {
        KUCOIN_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "KuCoin"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // KuCoin timestamp endpoint - test connectivity to the REST API
        let endpoint = "timestamp";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Kucoin {
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

        // Format symbol for KuCoin
        let kucoin_symbol = format_symbol_for_exchange(symbol, &CexExchange::Kucoin)?;

        // Get order book level 1 for bid/ask prices and quantities
        let book_endpoint = format!("market/orderbook/level1?symbol={}", kucoin_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&book_endpoint).await?;

        // Check if API returned success (KuCoin uses "200000" for success)
        let code = response["code"].as_str().ok_or_else(|| {
            MarketScannerError::ApiError("KuCoin API response missing code".to_string())
        })?;

        if code != "200000" {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "KuCoin API error: {} - {}",
                code, msg
            )));
        }

        // Check if data exists
        let data = response["data"].as_object().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "KuCoin API error: returned null or invalid data for symbol: {}",
                symbol
            ))
        })?;
        // Deserialize the order book data
        let order_book_data: types::KucoinOrderBookData =
            serde_json::from_value(serde_json::Value::Object(data.clone())).map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "KuCoin API error: failed to parse order book data: {}",
                    e
                ))
            })?;
        // Get best bid and ask from order book data
        let bid = parse_f64(&order_book_data.best_bid, "bid price")?;
        let ask = parse_f64(&order_book_data.best_ask, "ask price")?;
        let bid_qty = parse_f64(&order_book_data.best_bid_size, "bid quantity")?;
        let ask_qty = parse_f64(&order_book_data.best_ask_size, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert KuCoin symbol format (BTC-USDT) back to standard (BTCUSDT)
        let standard_symbol = kucoin_symbol.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Kucoin),
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

        // KuCoin supports up to 100 symbols per topic
        let kucoin_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Kucoin))
            .collect::<Result<Vec<_>, _>>()?;

        let client = self.client.clone();
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                // 1) Get WS endpoint via bullet-public (POST)
                let bullet_url = format!("{}/bullet-public", KUCOIN_API_BASE);
                let bullet_resp = client.post(&bullet_url).send().await;
                let bullet = match bullet_resp {
                    Ok(r) => match r.json::<KucoinBulletPublicResponse>().await {
                        Ok(b) => b,
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
                    },
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

                if bullet.code != "200000" {
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

                let server = match bullet.data.instance_servers.first() {
                    Some(s) => s,
                    None => {
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

                let connect_id = get_timestamp_millis();
                let ws_url = format!(
                    "{}?token={}&connectId={}",
                    server.endpoint, bullet.data.token, connect_id
                );

                // 2) Connect
                let (ws_stream, _) = match tokio_tungstenite::connect_async(&ws_url).await {
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

                let (mut write, mut read) = ws_stream.split();

                // 3) Subscribe (chunks of 100)
                for chunk in kucoin_symbols.chunks(100) {
                    let topic = format!("/spotMarket/level1:{}", chunk.join(","));
                    let sub_msg = serde_json::json!({
                        "id": connect_id,
                        "type": "subscribe",
                        "topic": topic,
                        "response": true
                    });
                    if write
                        .send(WsMessage::Text(sub_msg.to_string()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }

                // 4) Read loop + heartbeat
                let ping_every = std::time::Duration::from_millis(server.ping_interval.max(5000));
                let mut ping_interval = tokio::time::interval(ping_every);
                ping_interval.tick().await;

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
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
                                    let v: serde_json::Value = match serde_json::from_str(&t) {
                                        Ok(v) => v,
                                        Err(_) => continue,
                                    };

                                    // Server ping in JSON form: {"id":"...","type":"ping"}
                                    if v.get("type").and_then(|x| x.as_str()) == Some("ping") {
                                        let pong = serde_json::json!({
                                            "id": v.get("id").cloned().unwrap_or(serde_json::Value::from(connect_id)),
                                            "type": "pong"
                                        });
                                        let _ = write.send(WsMessage::Text(pong.to_string())).await;
                                        continue;
                                    }

                                    if v.get("type").and_then(|x| x.as_str()) != Some("message") {
                                        continue;
                                    }

                                    if v.get("subject").and_then(|x| x.as_str()) != Some("level1") {
                                        continue;
                                    }

                                    if let Some(price) = parse_kucoin_level1(&v) {
                                        if tx.send(price).await.is_err() {
                                            return;
                                        }
                                    }
                                }
                                WsMessage::Close(_) => break,
                                _ => {}
                            }
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

#[derive(Debug, Deserialize)]
struct KucoinBulletPublicResponse {
    code: String,
    data: KucoinBulletData,
}

#[derive(Debug, Deserialize)]
struct KucoinBulletData {
    token: String,
    #[serde(rename = "instanceServers")]
    instance_servers: Vec<KucoinInstanceServer>,
}

#[derive(Debug, Deserialize)]
struct KucoinInstanceServer {
    endpoint: String,
    #[serde(rename = "pingInterval")]
    ping_interval: u64,
    // pingTimeout is returned by API but not required for our client loop
    #[serde(rename = "pingTimeout")]
    _ping_timeout: u64,
}

fn parse_kucoin_level1(v: &serde_json::Value) -> Option<CexPrice> {
    let topic = v.get("topic")?.as_str()?;
    let symbol = topic.split(':').nth(1)?;
    let data = v.get("data")?;

    // data.asks = ["price","size"], data.bids = ["price","size"]
    let ask_arr = data.get("asks")?.as_array()?;
    let bid_arr = data.get("bids")?.as_array()?;
    if ask_arr.len() < 2 || bid_arr.len() < 2 {
        return None;
    }

    let ask_px = ask_arr[0].as_str()?;
    let ask_sz = ask_arr[1].as_str().unwrap_or("0");
    let bid_px = bid_arr[0].as_str()?;
    let bid_sz = bid_arr[1].as_str().unwrap_or("0");

    let bid = parse_f64(bid_px, "bid").ok()?;
    let ask = parse_f64(ask_px, "ask").ok()?;
    if bid <= 0.0 || ask <= 0.0 {
        return None;
    }

    let bid_qty = parse_f64(bid_sz, "bid_qty").unwrap_or(0.0);
    let ask_qty = parse_f64(ask_sz, "ask_qty").unwrap_or(0.0);
    let std_symbol = standard_symbol_for_cex_ws_response(symbol, &CexExchange::Kucoin);

    Some(CexPrice {
        symbol: std_symbol,
        mid_price: find_mid_price(bid, ask),
        bid_price: bid,
        ask_price: ask,
        bid_qty,
        ask_qty,
        timestamp: get_timestamp_millis(),
        exchange: Exchange::Cex(CexExchange::Kucoin),
    })
}
