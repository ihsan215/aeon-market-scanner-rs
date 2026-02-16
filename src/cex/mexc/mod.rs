mod types;

use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, parse_f64, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use prost::Message;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use types::{MexcBookTickerResponse, MexcPushBody, MexcPushDataWrapper};

const MEXC_API_BASE: &str = "https://api.mexc.com/api/v3";
const MEXC_WS_URL: &str = "wss://wbs-api.mexc.com/ws";

create_exchange!(Mexc);

#[async_trait]
impl ExchangeTrait for Mexc {
    fn api_base(&self) -> &str {
        MEXC_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Mexc"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // MEXC ping endpoint - test connectivity to the REST API
        let endpoint = "ping";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Mexc {
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

        // Format symbol for MEXC
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        let endpoint = format!("ticker/bookTicker?symbol={}", mexc_symbol);

        let ticker: MexcBookTickerResponse = self.get(&endpoint).await?;

        let bid = parse_f64(&ticker.bid_price, "bid price")?;
        let ask = parse_f64(&ticker.ask_price, "ask price")?;
        let mid_price = find_mid_price(bid, ask);
        let bid_qty = parse_f64(&ticker.bid_qty, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_qty, "ask quantity")?;

        // Normalize symbol to standard format
        let standard_symbol = normalize_symbol(&ticker.symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::MEXC),
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

        let mexc_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::MEXC))
            .collect::<Result<Vec<_>, _>>()?;

        // Subscribe: spot@public.aggre.bookTicker.v3.api.pb@100ms@SYMBOL
        let params: Vec<String> = mexc_symbols
            .iter()
            .map(|s| format!("spot@public.aggre.bookTicker.v3.api.pb@100ms@{}", s))
            .collect();
        let subscribe_msg = serde_json::json!({
            "method": "SUBSCRIPTION",
            "params": params
        });
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(MEXC_WS_URL).await {
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
                    .send(WsMessage::Text(subscribe_msg.to_string()))
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

                let mut ping_interval = tokio::time::interval(std::time::Duration::from_secs(15));
                ping_interval.tick().await;

                loop {
                    tokio::select! {
                        _ = ping_interval.tick() => {
                            let ping = serde_json::json!({"method": "PING"});
                            if write.send(WsMessage::Text(ping.to_string())).await.is_err() {
                                break;
                            }
                        }
                        msg = read.next() => {
                            let msg = match msg {
                                Some(Ok(m)) => m,
                                _ => break,
                            };
                            match msg {
                                WsMessage::Text(t) => {
                                    // JSON: subscribe ack, PONG, error
                                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                                        if v.get("msg").and_then(|m| m.as_str()) == Some("PONG") {
                                            continue;
                                        }
                                        if v.get("code").is_some() || v.get("msg").is_some() {
                                            continue; // ack or other control
                                        }
                                    }
                                }
                                WsMessage::Binary(b) => {
                                    if let Some(price) = parse_mexc_protobuf(&b) {
                                        if tx.send(price).await.is_err() {
                                            return;
                                        }
                                    }
                                }
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

fn parse_mexc_protobuf(bytes: &[u8]) -> Option<CexPrice> {
    let wrapper = MexcPushDataWrapper::decode(prost::bytes::Bytes::copy_from_slice(bytes)).ok()?;
    let body = wrapper.body?;
    let ticker = match body {
        MexcPushBody::PublicAggreBookTicker(t) => t,
    };

    let bid = parse_f64(&ticker.bid_price, "bid").ok()?;
    let ask = parse_f64(&ticker.ask_price, "ask").ok()?;
    if bid <= 0.0 || ask <= 0.0 {
        return None;
    }

    // symbol from wrapper or parse from channel (spot@...@100ms@BTCUSDT)
    let symbol = wrapper
        .symbol
        .as_deref()
        .filter(|s| !s.is_empty())
        .or_else(|| wrapper.channel.rsplit('@').next().filter(|s| !s.is_empty()))
        .unwrap_or("");
    if symbol.is_empty() {
        return None;
    }
    let standard_symbol = standard_symbol_for_cex_ws_response(symbol, &CexExchange::MEXC);

    Some(CexPrice {
        symbol: standard_symbol,
        mid_price: find_mid_price(bid, ask),
        bid_price: bid,
        ask_price: ask,
        bid_qty: parse_f64(&ticker.bid_quantity, "bid_qty").unwrap_or(0.0),
        ask_qty: parse_f64(&ticker.ask_quantity, "ask_qty").unwrap_or(0.0),
        timestamp: get_timestamp_millis(),
        exchange: Exchange::Cex(CexExchange::MEXC),
    })
}
