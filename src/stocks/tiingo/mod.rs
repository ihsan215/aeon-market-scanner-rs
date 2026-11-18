mod types;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::common::utils::iso8601_ms;
use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::stocks::tiingo::types::TiingoIexQuote;
use crate::stocks::{
    DEFAULT_MAX_SPREAD, StockPrice, StockProvider, StockTrait, credential_from_env,
    normalize_stock_symbol, normalize_stock_symbols, usable_quote_mid,
};

const TIINGO_API_BASE: &str = "https://api.tiingo.com";

/// Tickers sent per `/iex` request. The S&P 500 fits in 6 requests.
const MAX_SYMBOLS_PER_REQUEST: usize = 100;

/// Environment variable read by [`Tiingo::from_env`].
pub const TIINGO_API_TOKEN_ENV: &str = "TIINGO_API_TOKEN";

/// US stock prices from the [Tiingo](https://www.tiingo.com) IEX endpoint (`/iex`).
///
/// - Needs a free API token (`TIINGO_API_TOKEN`). The free Starter plan allows 50 requests
///   per hour, 1,000 per day and **500 unique symbols per month**, so the full S&P 500
///   (503 tickers) does not fit in one month on Starter.
/// - Without an IEX market data agreement Tiingo returns no bid/ask; prices are then the
///   last price (`tngoLast`). With one, bid/ask are filled and `price` is their midpoint,
///   as long as the quote is not crossed and at most [`with_max_spread`](Self::with_max_spread)
///   wide (2% by default); otherwise the last price is used.
/// - Batched: [`get_prices`](StockTrait::get_prices) sends up to 100 tickers per request.
pub struct Tiingo {
    client: reqwest::Client,
    token: String,
    max_spread: f64,
}

impl Tiingo {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: create_http_client(),
            token: token.into(),
            max_spread: DEFAULT_MAX_SPREAD,
        }
    }

    /// Widest bid/ask spread, as a fraction of mid (default [`DEFAULT_MAX_SPREAD`], 2%),
    /// still used as the price. Only matters with an IEX data agreement.
    pub fn with_max_spread(mut self, max_spread: f64) -> Self {
        self.max_spread = max_spread;
        self
    }

    /// Reads the token from `TIINGO_API_TOKEN`.
    pub fn from_env() -> Result<Self, MarketScannerError> {
        Ok(Self::new(credential_from_env(
            TIINGO_API_TOKEN_ENV,
            "Tiingo",
        )?))
    }

    /// Quotes keyed by standard ticker (`BRK.B`).
    async fn fetch_quotes(
        &self,
        symbols: &[String],
    ) -> Result<HashMap<String, TiingoIexQuote>, MarketScannerError> {
        // Tiingo writes class shares with a dash (`brk-b`).
        let tickers = symbols
            .iter()
            .map(|s| s.replace('.', "-").to_lowercase())
            .collect::<Vec<_>>()
            .join(",");
        let response = self
            .client
            .get(format!("{TIINGO_API_BASE}/iex/"))
            .header("Authorization", format!("Token {}", self.token))
            .query(&[("tickers", tickers)])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            let hint = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                " (rate limit reached; retry later)"
            } else {
                ""
            };
            return Err(MarketScannerError::ApiError(format!(
                "Tiingo API error: {status}{hint} - {body}"
            )));
        }

        let quotes: Vec<TiingoIexQuote> = response.json().await?;
        Ok(quotes
            .into_iter()
            .filter_map(|q| Some((normalize_stock_symbol(&q.ticker).ok()?, q)))
            .collect())
    }
}

#[async_trait]
impl StockTrait for Tiingo {
    fn provider(&self) -> StockProvider {
        StockProvider::Tiingo
    }

    fn provider_name(&self) -> &str {
        "Tiingo"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.fetch_quotes(&["SPY".to_string()]).await.map(|_| ())
    }

    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
        let symbol = normalize_stock_symbol(symbol)?;
        let quotes = self.fetch_quotes(std::slice::from_ref(&symbol)).await?;
        quotes
            .get(&symbol)
            .and_then(|q| to_stock_price(&symbol, q, self.max_spread))
            .ok_or_else(|| {
                MarketScannerError::InvalidSymbol(format!("Tiingo has no price for {symbol}"))
            })
    }

    /// Batched: up to 100 tickers per request, requests sent concurrently.
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<StockPrice>, MarketScannerError> {
        let normalized = normalize_stock_symbols(symbols);
        let responses = futures::future::try_join_all(
            normalized
                .chunks(MAX_SYMBOLS_PER_REQUEST)
                .map(|chunk| self.fetch_quotes(chunk)),
        )
        .await?;

        let mut quotes = HashMap::new();
        for response in responses {
            quotes.extend(response);
        }
        Ok(normalized
            .iter()
            .filter_map(|symbol| to_stock_price(symbol, quotes.get(symbol)?, self.max_spread))
            .collect())
    }
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

/// Bid/ask midpoint for a usable quote (see [`usable_quote_mid`]), otherwise the last price.
fn to_stock_price(symbol: &str, quote: &TiingoIexQuote, max_spread: f64) -> Option<StockPrice> {
    let last = positive(quote.last).or(positive(quote.tngo_last));
    let mid = match (quote.bid_price, quote.ask_price) {
        (Some(bid), Some(ask)) => usable_quote_mid(bid, ask, max_spread),
        _ => None,
    };
    let has_quote = mid.is_some();

    let (price, time) = match mid {
        Some(mid) => (mid, quote.quote_timestamp.as_deref()),
        None => (last?, quote.last_sale_timestamp.as_deref()),
    };
    let exchange_timestamp = time.or(quote.timestamp.as_deref()).and_then(iso8601_ms);

    Some(StockPrice {
        symbol: symbol.to_string(),
        price,
        bid_price: quote.bid_price.filter(|_| has_quote),
        ask_price: quote.ask_price.filter(|_| has_quote),
        bid_qty: quote.bid_size.filter(|_| has_quote),
        ask_qty: quote.ask_size.filter(|_| has_quote),
        last_price: last,
        timestamp: get_timestamp_millis(),
        exchange_timestamp,
        provider: StockProvider::Tiingo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_price_without_iex_entitlement() {
        let json = r#"[{"ticker":"AAPL","timestamp":"2026-11-17T21:00:00+00:00",
            "quoteTimestamp":null,"lastSaleTimestamp":null,"last":null,"lastSize":null,
            "tngoLast":351.91,"prevClose":349.81,"open":349.5,"high":353.0,"low":348.2,
            "mid":null,"volume":0,"bidSize":null,"bidPrice":null,"askSize":null,"askPrice":null}]"#;
        let quotes: Vec<TiingoIexQuote> = serde_json::from_str(json).unwrap();
        let price = to_stock_price("AAPL", &quotes[0], DEFAULT_MAX_SPREAD).unwrap();
        assert_eq!(price.price, 351.91);
        assert_eq!(price.last_price, Some(351.91));
        assert!(price.bid_price.is_none() && price.ask_price.is_none());
        assert_eq!(price.exchange_timestamp, Some(1_794_949_200_000));
    }

    #[test]
    fn mid_price_with_bid_ask() {
        let json = r#"[{"ticker":"brk-b","timestamp":"2026-11-17T20:59:59.5-05:00",
            "quoteTimestamp":"2026-11-17T15:59:59.123456789-05:00","last":586.7,
            "tngoLast":586.7,"bidPrice":586.6,"bidSize":100,"askPrice":586.8,"askSize":200}]"#;
        let quotes: Vec<TiingoIexQuote> = serde_json::from_str(json).unwrap();
        assert_eq!(normalize_stock_symbol(&quotes[0].ticker).unwrap(), "BRK.B");
        let price = to_stock_price("BRK.B", &quotes[0], DEFAULT_MAX_SPREAD).unwrap();
        assert!((price.price - 586.7).abs() < 1e-9);
        assert_eq!(price.bid_price, Some(586.6));
        assert_eq!(price.ask_qty, Some(200.0));
        assert_eq!(price.last_price, Some(586.7));
        assert_eq!(price.exchange_timestamp, Some(1_794_949_199_123));
    }

    #[test]
    fn wide_or_crossed_quote_falls_back_to_last() {
        for (bid, ask) in [(478.04, 530.96), (101.0, 99.0)] {
            let quote = TiingoIexQuote {
                ticker: "MSFT".to_string(),
                tngo_last: Some(506.07),
                bid_price: Some(bid),
                ask_price: Some(ask),
                bid_size: Some(40.0),
                ask_size: Some(40.0),
                ..Default::default()
            };
            let price = to_stock_price("MSFT", &quote, DEFAULT_MAX_SPREAD).unwrap();
            assert_eq!(price.price, 506.07, "bid={bid} ask={ask}");
            assert!(price.bid_price.is_none() && price.ask_qty.is_none());
        }
    }

    #[test]
    fn no_price_at_all() {
        let quote = TiingoIexQuote {
            ticker: "XYZ".to_string(),
            ..Default::default()
        };
        assert!(to_stock_price("XYZ", &quote, DEFAULT_MAX_SPREAD).is_none());
    }
}
