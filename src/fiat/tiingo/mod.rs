mod types;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::common::utils::iso8601_ms;
use crate::common::{MarketScannerError, create_http_client, find_mid_price, get_timestamp_millis};
use crate::fiat::tiingo::types::TiingoFxQuote;
use crate::fiat::{FiatProvider, FiatRate, FiatTrait, parse_fiat_pair};
use crate::stocks::{TIINGO_API_TOKEN_ENV, credential_from_env};

const TIINGO_FX_TOP_URL: &str = "https://api.tiingo.com/tiingo/fx/top";

/// Pairs sent per `/tiingo/fx/top` request.
const MAX_PAIRS_PER_REQUEST: usize = 100;

/// Real-time fiat rates with bid/ask from [Tiingo](https://www.tiingo.com/products/forex-api)
/// (`/tiingo/fx/top`).
///
/// - Uses the same token as the stock provider [`crate::stocks::Tiingo`] (`TIINGO_API_TOKEN`)
///   and the same account quota: on the free Starter plan 50 requests per hour, 1,000 per day
///   and 500 unique symbols per month, shared between stocks and currency pairs.
/// - Quotes are top-of-book from the interbank market, usually seconds old for major pairs;
///   [`FiatRate::exchange_timestamp`] is the quote time and [`FiatRate::bid`] / [`FiatRate::ask`]
///   are filled.
/// - [`get_rates`](FiatTrait::get_rates) sends many pairs in one request. Pairs Tiingo only
///   lists the other way round (`TRY/USD` when it has `USD/TRY`) are fetched in a second request
///   and inverted.
/// - About 140 pairs; some exotic ones (e.g. `USD/ARS`) are not covered and give `InvalidSymbol`.
/// - The free plan is for personal / internal use; displaying the data publicly needs a
///   commercial licence.
pub struct TiingoFiat {
    client: reqwest::Client,
    token: String,
}

impl TiingoFiat {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            client: create_http_client(),
            token: token.into(),
        }
    }

    /// Reads the token from `TIINGO_API_TOKEN`.
    pub fn from_env() -> Result<Self, MarketScannerError> {
        Ok(Self::new(credential_from_env(
            TIINGO_API_TOKEN_ENV,
            "Tiingo",
        )?))
    }

    /// Top-of-book quotes keyed by lowercase ticker (`eurusd`). Missing tickers are left out.
    async fn fetch_top(
        &self,
        tickers: &[String],
    ) -> Result<HashMap<String, TiingoFxQuote>, MarketScannerError> {
        let mut quotes = HashMap::new();
        for chunk in tickers.chunks(MAX_PAIRS_PER_REQUEST) {
            let response = self
                .client
                .get(TIINGO_FX_TOP_URL)
                .header("Authorization", format!("Token {}", self.token))
                .query(&[("tickers", chunk.join(","))])
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
                    "Tiingo FX API error: {status}{hint} - {body}"
                )));
            }

            let batch: Vec<TiingoFxQuote> = response.json().await?;
            quotes.extend(batch.into_iter().map(|q| (q.ticker.to_lowercase(), q)));
        }
        Ok(quotes)
    }
}

fn ticker(base: &str, quote: &str) -> String {
    format!("{base}{quote}").to_lowercase()
}

#[async_trait]
impl FiatTrait for TiingoFiat {
    fn provider(&self) -> FiatProvider {
        FiatProvider::Tiingo
    }

    fn provider_name(&self) -> &str {
        "Tiingo FX"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_rate("EUR/USD").await.map(|_| ())
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let mut rates = self.get_rates(&[pair]).await?;
        rates.pop().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("Tiingo FX has no rate for {pair}"))
        })
    }

    /// One request for all pairs (100 per request), plus one more for pairs that only exist
    /// inverted. Results keep input order; fails if any pair is not covered.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        let parsed = pairs
            .iter()
            .map(|pair| parse_fiat_pair(pair))
            .collect::<Result<Vec<_>, _>>()?;

        let mut direct: Vec<String> = parsed.iter().map(|(b, q)| ticker(b, q)).collect();
        direct.sort();
        direct.dedup();
        let mut quotes = self.fetch_top(&direct).await?;

        let mut inverse: Vec<String> = parsed
            .iter()
            .filter(|(b, q)| !quotes.contains_key(&ticker(b, q)))
            .map(|(b, q)| ticker(q, b))
            .collect();
        inverse.sort();
        inverse.dedup();
        if !inverse.is_empty() {
            quotes.extend(self.fetch_top(&inverse).await?);
        }

        let fetched_at = get_timestamp_millis();
        parsed
            .iter()
            .map(|(base, quote)| {
                let rate = match quotes.get(&ticker(base, quote)) {
                    Some(q) => to_rate(base, quote, q, false, fetched_at),
                    None => quotes
                        .get(&ticker(quote, base))
                        .and_then(|q| to_rate(base, quote, q, true, fetched_at)),
                };
                rate.ok_or_else(|| {
                    MarketScannerError::InvalidSymbol(format!(
                        "Tiingo FX has no rate for {base}/{quote}"
                    ))
                })
            })
            .collect()
    }
}

fn positive(value: Option<f64>) -> Option<f64> {
    value.filter(|v| v.is_finite() && *v > 0.0)
}

/// `inverted`: `q` is the quote for `quote/base`, so flip it (bid = 1/ask, ask = 1/bid).
fn to_rate(
    base: &str,
    quote: &str,
    q: &TiingoFxQuote,
    inverted: bool,
    fetched_at: u64,
) -> Option<FiatRate> {
    let book = match (positive(q.bid_price), positive(q.ask_price)) {
        (Some(bid), Some(ask)) if ask >= bid => Some((bid, ask)),
        _ => None,
    };
    let book = book.map(|(bid, ask)| {
        if inverted {
            (1.0 / ask, 1.0 / bid)
        } else {
            (bid, ask)
        }
    });

    let rate = match book {
        Some((bid, ask)) => find_mid_price(bid, ask),
        None => {
            let mid = positive(q.mid_price)?;
            if inverted { 1.0 / mid } else { mid }
        }
    };

    let exchange_timestamp = q.quote_timestamp.as_deref().and_then(iso8601_ms);
    let date = exchange_timestamp
        .and_then(|ms| chrono::DateTime::from_timestamp_millis(i64::try_from(ms).ok()?))
        .map(|t| t.format("%Y-%m-%d").to_string())
        .unwrap_or_default();

    Some(FiatRate {
        pair: format!("{base}/{quote}"),
        base: base.to_string(),
        quote: quote.to_string(),
        rate,
        bid: book.map(|(bid, _)| bid),
        ask: book.map(|(_, ask)| ask),
        date,
        timestamp: fetched_at,
        exchange_timestamp,
        provider: FiatProvider::Tiingo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn parses_both_ticker_field_names() {
        let json = r#"[
            {"ticker":"eurusd","quoteTimestamp":"2026-11-18T08:52:51.681000+00:00","bidPrice":1.09619,"bidSize":1000000.0,"askPrice":1.0962,"askSize":1000000.0,"midPrice":1.096195},
            {"index":"usdtry","quoteTimestamp":"2026-11-18T08:45:49.477000+00:00","bidPrice":50.44251,"bidSize":1000000.0,"askPrice":50.44311,"askSize":1000000.0,"midPrice":50.44281}
        ]"#;
        let quotes: Vec<TiingoFxQuote> = serde_json::from_str(json).unwrap();
        assert_eq!(quotes[0].ticker, "eurusd");
        assert_eq!(quotes[1].ticker, "usdtry");

        let rate = to_rate("EUR", "USD", &quotes[0], false, 1).unwrap();
        assert_eq!(rate.pair, "EUR/USD");
        assert_eq!(rate.bid, Some(1.09619));
        assert_eq!(rate.ask, Some(1.0962));
        assert!(approx(rate.rate, 1.096195));
        assert_eq!(rate.date, "2026-11-18");
        assert_eq!(rate.exchange_timestamp, Some(1_794_991_971_681));
        assert_eq!(rate.timestamp, 1);
        assert_eq!(rate.provider, FiatProvider::Tiingo);
    }

    #[test]
    fn inverts_quotes() {
        let q = TiingoFxQuote {
            ticker: "usdtry".to_string(),
            bid_price: Some(50.0),
            ask_price: Some(50.5),
            mid_price: Some(50.25),
            ..Default::default()
        };
        let rate = to_rate("TRY", "USD", &q, true, 0).unwrap();
        assert_eq!(rate.pair, "TRY/USD");
        assert!(approx(rate.bid.unwrap(), 1.0 / 50.5));
        assert!(approx(rate.ask.unwrap(), 1.0 / 50.0));
        assert!(rate.bid.unwrap() < rate.rate && rate.rate < rate.ask.unwrap());
    }

    #[test]
    fn falls_back_to_mid_without_a_valid_book() {
        let q = TiingoFxQuote {
            ticker: "eurusd".to_string(),
            bid_price: Some(1.2),
            ask_price: Some(1.1),
            mid_price: Some(1.15),
            ..Default::default()
        };
        let rate = to_rate("EUR", "USD", &q, false, 0).unwrap();
        assert_eq!(rate.rate, 1.15);
        assert!(rate.bid.is_none() && rate.ask.is_none());

        let empty = TiingoFxQuote::default();
        assert!(to_rate("EUR", "USD", &empty, false, 0).is_none());
    }
}
