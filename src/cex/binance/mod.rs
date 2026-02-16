mod types;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, parse_f64, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::StreamExt;
use tokio::sync::mpsc;
use types::{BinanceBookTickerResponse, BinanceBookTickerWs};

const BINANCE_API_BASE: &str = "https://api.binance.com/api/v3";
const BINANCE_WS_BASE: &str = "wss://stream.binance.com:9443";

create_exchange!(Binance);

#[async_trait]
impl ExchangeTrait for Binance {
    fn api_base(&self) -> &str {
        BINANCE_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Binance"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Binance ping endpoint - test connectivity to the REST API
        let endpoint = "ping";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Binance {
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

        // Format symbol for Binance
        let binance_symbol = format_symbol_for_exchange(symbol, &CexExchange::Binance)?;
        let endpoint = format!("ticker/bookTicker?symbol={}", binance_symbol);

        let ticker: BinanceBookTickerResponse = self.get(&endpoint).await?;

        let bid = parse_f64(&ticker.bid_price, "bid price")?;
        let ask = parse_f64(&ticker.ask_price, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid_qty, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_qty, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

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
            exchange: Exchange::Cex(CexExchange::Binance),
        })
    }

    /// Connection stays open; incoming prices are sent over the returned Receiver.
    /// When the channel closes (Receiver returns None), the connection has closed.
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

        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| {
                let sym = format_symbol_for_exchange_ws(s, &CexExchange::Binance).ok()?;
                Some(format!("{}@bookTicker", sym.to_lowercase()))
            })
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| MarketScannerError::InvalidSymbol("Invalid symbol".to_string()))?;

        let is_combined = stream_names.len() > 1;
        let url = if stream_names.len() == 1 {
            format!("{}/ws/{}", BINANCE_WS_BASE, stream_names[0])
        } else {
            format!(
                "{}/stream?streams={}",
                BINANCE_WS_BASE,
                stream_names.join("/")
            )
        };

        let single_symbol = if symbols.len() == 1 {
            Some(standard_symbol_for_cex_ws_response(
                symbols[0],
                &CexExchange::Binance,
            ))
        } else {
            None
        };
        let (tx, rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut backoff = std::time::Duration::from_secs(1);
            let max_backoff = std::time::Duration::from_secs(30);
            let mut attempts: u32 = 0;

            loop {
                let (ws_stream, _) = match tokio_tungstenite::connect_async(&url).await {
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

                    // Combined stream: {"stream":"btcusdt@bookTicker","data":{...}}
                    // Single stream: raw payload {b, B, a, A}
                    let (ticker_value, symbol_std) = if is_combined {
                        let stream = match value.get("stream").and_then(|s| s.as_str()) {
                            Some(s) => s,
                            None => continue,
                        };
                        let data = match value.get("data") {
                            Some(d) => d.clone(),
                            None => continue,
                        };
                        let sym = stream.split('@').next().unwrap_or("btcusdt");
                        (
                            data,
                            standard_symbol_for_cex_ws_response(sym, &CexExchange::Binance),
                        )
                    } else {
                        (
                            value,
                            single_symbol.clone().unwrap_or_else(|| {
                                standard_symbol_for_cex_ws_response(
                                    "btcusdt",
                                    &CexExchange::Binance,
                                )
                            }),
                        )
                    };

                    let ticker: BinanceBookTickerWs = match serde_json::from_value(ticker_value) {
                        Ok(t) => t,
                        Err(_) => continue,
                    };

                    let (bid, ask, bid_qty, ask_qty) = match (
                        parse_f64(&ticker.b, "bid"),
                        parse_f64(&ticker.a, "ask"),
                        parse_f64(&ticker.B, "bidQty"),
                        parse_f64(&ticker.A, "askQty"),
                    ) {
                        (Ok(b), Ok(a), Ok(bq), Ok(aq)) => (b, a, bq, aq),
                        _ => continue,
                    };
                    let price = CexPrice {
                        symbol: symbol_std,
                        mid_price: find_mid_price(bid, ask),
                        bid_price: bid,
                        ask_price: ask,
                        bid_qty,
                        ask_qty,
                        timestamp: get_timestamp_millis(),
                        exchange: Exchange::Cex(CexExchange::Binance),
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
