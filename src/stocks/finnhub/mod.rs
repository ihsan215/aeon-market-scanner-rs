mod types;

use async_trait::async_trait;

use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::stocks::finnhub::types::FinnhubQuote;
use crate::stocks::{
    StockPrice, StockProvider, StockTrait, credential_from_env, normalize_stock_symbol,
};

const FINNHUB_API_BASE: &str = "https://finnhub.io/api/v1";

/// Environment variable read by [`Finnhub::from_env`].
pub const FINNHUB_API_KEY_ENV: &str = "FINNHUB_API_KEY";

/// US stock prices from [Finnhub](https://finnhub.io) (`/api/v1/quote`).
///
/// - Needs a free API key (`FINNHUB_API_KEY`). The free plan allows about 60 requests per
///   minute and is for personal use.
/// - Returns the last price only: bid/ask fields are `None`.
/// - No batch endpoint: [`get_prices`](StockTrait::get_prices) sends one request per ticker,
///   5 at a time, so the S&P 500 does not fit in one minute's limit. Expect a rate-limit
///   error for long lists.
pub struct Finnhub {
    client: reqwest::Client,
    api_key: String,
}

impl Finnhub {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            client: create_http_client(),
            api_key: api_key.into(),
        }
    }

    /// Reads the key from `FINNHUB_API_KEY`.
    pub fn from_env() -> Result<Self, MarketScannerError> {
        Ok(Self::new(credential_from_env(
            FINNHUB_API_KEY_ENV,
            "Finnhub",
        )?))
    }
}

#[async_trait]
impl StockTrait for Finnhub {
    fn provider(&self) -> StockProvider {
        StockProvider::Finnhub
    }

    fn provider_name(&self) -> &str {
        "Finnhub"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_price("SPY").await.map(|_| ())
    }

    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
        let symbol = normalize_stock_symbol(symbol)?;
        let response = self
            .client
            .get(format!("{FINNHUB_API_BASE}/quote"))
            .header("X-Finnhub-Token", &self.api_key)
            .query(&[("symbol", symbol.as_str())])
            .send()
            .await?;

        let status = response.status();
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MarketScannerError::ApiError(
                "Finnhub rate limit reached (HTTP 429); retry later".to_string(),
            ));
        }
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MarketScannerError::ApiError(format!(
                "Finnhub API error: {status} - {body}"
            )));
        }

        let quote: FinnhubQuote = response.json().await?;
        to_stock_price(&symbol, &quote).ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("Finnhub has no quote for {symbol}"))
        })
    }
}

/// `None` for the all-zero response Finnhub sends for unknown symbols.
fn to_stock_price(symbol: &str, quote: &FinnhubQuote) -> Option<StockPrice> {
    if quote.c <= 0.0 {
        return None;
    }
    Some(StockPrice {
        symbol: symbol.to_string(),
        price: quote.c,
        bid_price: None,
        ask_price: None,
        bid_qty: None,
        ask_qty: None,
        last_price: Some(quote.c),
        timestamp: get_timestamp_millis(),
        exchange_timestamp: u64::try_from(quote.t)
            .ok()
            .filter(|t| *t > 0)
            .map(|t| t * 1000),
        provider: StockProvider::Finnhub,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quote() {
        let json = r#"{"c":351.91,"d":2.1,"dp":0.6,"h":353.0,"l":348.2,"o":349.5,"pc":349.81,"t":1794949201}"#;
        let quote: FinnhubQuote = serde_json::from_str(json).unwrap();
        let price = to_stock_price("AAPL", &quote).unwrap();
        assert_eq!(price.price, 351.91);
        assert_eq!(price.last_price, Some(351.91));
        assert!(price.bid_price.is_none());
        assert_eq!(price.exchange_timestamp, Some(1_794_949_201_000));
        assert_eq!(price.provider, StockProvider::Finnhub);
    }

    #[test]
    fn unknown_symbol_is_all_zero() {
        let json = r#"{"c":0,"d":null,"dp":null,"h":0,"l":0,"o":0,"pc":0,"t":0}"#;
        let quote: FinnhubQuote = serde_json::from_str(json).unwrap();
        assert!(to_stock_price("XYZ", &quote).is_none());
    }
}
