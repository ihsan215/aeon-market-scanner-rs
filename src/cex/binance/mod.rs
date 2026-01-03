mod types;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;
use types::BinanceBookTickerResponse;

const BINANCE_API_BASE: &str = "https://api.binance.com/api/v3";

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
        let endpoint = "ping";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        let endpoint = format!("ticker/bookTicker?symbol={}", symbol.to_uppercase());

        let ticker: BinanceBookTickerResponse = self.get(&endpoint).await?;

        let bid = parse_f64(&ticker.bid_price, "bid price")?;
        let ask = parse_f64(&ticker.ask_price, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid_qty, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_qty, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

        Ok(CexPrice {
            symbol: ticker.symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Binance),
        })
    }
}
