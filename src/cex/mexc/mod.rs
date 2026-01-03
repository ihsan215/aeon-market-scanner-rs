mod types;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;
use types::MexcBookTickerResponse;

const MEXC_API_BASE: &str = "https://api.mexc.com/api/v3";

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
        let endpoint = "ping";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }
        let endpoint = format!("ticker/bookTicker?symbol={}", symbol.to_uppercase());

        let ticker: MexcBookTickerResponse = self.get(&endpoint).await?;

        let bid = parse_f64(&ticker.bid_price, "bid price")?;
        let ask = parse_f64(&ticker.ask_price, "ask price")?;
        let mid_price = find_mid_price(bid, ask);
        let bid_qty = parse_f64(&ticker.bid_qty, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_qty, "ask quantity")?;

        Ok(CexPrice {
            symbol: ticker.symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::MEXC),
        })
    }
}
