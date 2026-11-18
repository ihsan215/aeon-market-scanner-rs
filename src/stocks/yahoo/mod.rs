//! **Unofficial** Yahoo Finance stock provider, behind the `unofficial-yahoo` Cargo feature.

use async_trait::async_trait;

use crate::common::MarketScannerError;
use crate::common::get_timestamp_millis;
use crate::common::yahoo::{YAHOO_MAX_CONCURRENT_REQUESTS, YahooClient, YahooMeta, yahoo_time};
use crate::stocks::{
    StockPrice, StockProvider, StockTrait, get_prices_concurrently, normalize_stock_symbol,
};

/// US stock prices from Yahoo Finance. No API key needed.
///
/// **This is not an official API.** Yahoo does not document the endpoint, its terms do not
/// permit automated access, and it can rate limit (HTTP 429), block or change without notice.
/// Use it for personal use and prototypes, not as the only source in production, and do not
/// re-publish the data. Enable it with the `unofficial-yahoo` feature.
///
/// - Returns the last price only (bid/ask are `None`); stocks and ETFs only.
/// - One request per ticker, 4 at a time. Long lists such as the full S&P 500 are likely to
///   hit Yahoo's rate limit; [`get_prices`](StockTrait::get_prices) then returns an error.
pub struct YahooStocks {
    client: YahooClient,
}

impl Default for YahooStocks {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooStocks {
    pub fn new() -> Self {
        Self {
            client: YahooClient::new(),
        }
    }
}

#[async_trait]
impl StockTrait for YahooStocks {
    fn provider(&self) -> StockProvider {
        StockProvider::Yahoo
    }

    fn provider_name(&self) -> &str {
        "Yahoo Finance (unofficial)"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_price("SPY").await.map(|_| ())
    }

    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
        let symbol = normalize_stock_symbol(symbol)?;
        // Yahoo writes class shares with a dash (`BRK-B`).
        let meta = self
            .client
            .fetch_meta(&symbol.replace('.', "-"), &symbol)
            .await?;
        to_stock_price(&symbol, &meta).ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("Yahoo has no stock price for {symbol}"))
        })
    }

    /// One request per ticker with up to 4 in flight. Unknown tickers are left out; other
    /// errors, including Yahoo's rate limit, are returned.
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<StockPrice>, MarketScannerError> {
        get_prices_concurrently(self, symbols, YAHOO_MAX_CONCURRENT_REQUESTS).await
    }
}

fn to_stock_price(symbol: &str, meta: &YahooMeta) -> Option<StockPrice> {
    if !matches!(meta.instrument_type.as_deref(), Some("EQUITY" | "ETF")) {
        return None;
    }
    let price = meta.regular_market_price.filter(|p| *p > 0.0)?;
    let (_, exchange_timestamp) = yahoo_time(meta.regular_market_time);
    Some(StockPrice {
        symbol: symbol.to_string(),
        price,
        bid_price: None,
        ask_price: None,
        bid_qty: None,
        ask_qty: None,
        last_price: Some(price),
        timestamp: get_timestamp_millis(),
        exchange_timestamp,
        provider: StockProvider::Yahoo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_price_from_meta() {
        let meta = YahooMeta {
            regular_market_price: Some(586.76),
            regular_market_time: Some(1_794_949_318),
            instrument_type: Some("EQUITY".to_string()),
        };
        let price = to_stock_price("BRK.B", &meta).unwrap();
        assert_eq!(price.price, 586.76);
        assert_eq!(price.last_price, Some(586.76));
        assert_eq!(price.exchange_timestamp, Some(1_794_949_318_000));
        assert_eq!(price.provider, StockProvider::Yahoo);
    }

    #[test]
    fn rejects_non_stock_instruments() {
        for kind in ["CURRENCY", "INDEX", "CRYPTOCURRENCY"] {
            let meta = YahooMeta {
                regular_market_price: Some(1.0),
                instrument_type: Some(kind.to_string()),
                ..Default::default()
            };
            assert!(to_stock_price("X", &meta).is_none(), "{kind}");
        }
    }
}
