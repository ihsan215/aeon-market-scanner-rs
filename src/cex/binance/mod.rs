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
const BINANCE_WS_BASE: &str = "wss://stream.binance.com:9443/ws";

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
        symbol: &str,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        let binance_symbol = format_symbol_for_exchange_ws(symbol, &CexExchange::Binance)?;
        let stream_name = format!("{}@bookTicker", binance_symbol);
        let url = format!("{}/{}", BINANCE_WS_BASE, stream_name);

        let (ws_stream, _) = tokio_tungstenite::connect_async(&url).await.map_err(|e| {
            MarketScannerError::ApiError(format!("Binance WebSocket connect: {}", e))
        })?;

        let (_write, mut read) = ws_stream.split();
        let (tx, rx) = mpsc::channel(64);
        let symbol_std = standard_symbol_for_cex_ws_response(symbol, &CexExchange::Binance);

        tokio::spawn(async move {
            while let Some(Ok(msg)) = read.next().await {
                let text = match msg.into_text() {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let ticker: BinanceBookTickerWs = match serde_json::from_str(&text) {
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
                    symbol: symbol_std.clone(),
                    mid_price: find_mid_price(bid, ask),
                    bid_price: bid,
                    ask_price: ask,
                    bid_qty,
                    ask_qty,
                    timestamp: get_timestamp_millis(),
                    exchange: Exchange::Cex(CexExchange::Binance),
                };
                if tx.send(price).await.is_err() {
                    break;
                }
            }
        });

        Ok(rx)
    }
}
