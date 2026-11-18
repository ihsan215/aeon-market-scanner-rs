//! **Unofficial** Yahoo Finance fiat provider, behind the `unofficial-yahoo` Cargo feature.

use async_trait::async_trait;

use crate::common::MarketScannerError;
use crate::common::get_timestamp_millis;
use crate::common::yahoo::{YAHOO_MAX_CONCURRENT_REQUESTS, YahooClient, YahooMeta, yahoo_time};
use crate::fiat::{FiatProvider, FiatRate, FiatTrait, get_rates_concurrently, parse_fiat_pair};

/// Near-live fiat rates from Yahoo Finance (`EURUSD=X`, `USDTRY=X`, ...). No API key needed.
///
/// **This is not an official API.** Yahoo does not document the endpoint, its terms do not
/// permit automated access, and it can rate limit (HTTP 429), block or change without notice.
/// Use it for personal use and prototypes, not as the only source in production, and do not
/// re-publish the data. Enable it with the `unofficial-yahoo` feature.
///
/// - Covers practically every currency pair, with prices typically under a minute old.
/// - Returns the last price only: [`FiatRate::bid`] / [`FiatRate::ask`] are `None`.
/// - [`FiatRate::exchange_timestamp`] is the time of that price.
pub struct YahooFiat {
    client: YahooClient,
}

impl Default for YahooFiat {
    fn default() -> Self {
        Self::new()
    }
}

impl YahooFiat {
    pub fn new() -> Self {
        Self {
            client: YahooClient::new(),
        }
    }
}

#[async_trait]
impl FiatTrait for YahooFiat {
    fn provider(&self) -> FiatProvider {
        FiatProvider::Yahoo
    }

    fn provider_name(&self) -> &str {
        "Yahoo Finance (unofficial)"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_rate("EUR/USD").await.map(|_| ())
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let (base, quote) = parse_fiat_pair(pair)?;
        let label = format!("{base}/{quote}");
        let meta = self
            .client
            .fetch_meta(&format!("{base}{quote}=X"), &label)
            .await?;
        to_rate(&base, &quote, &meta).ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("Yahoo has no currency rate for {label}"))
        })
    }

    /// One request per pair (Yahoo's batch quote endpoint needs a session token) with up to 4
    /// in flight. Results keep input order; fails if any pair fails.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        get_rates_concurrently(self, pairs, YAHOO_MAX_CONCURRENT_REQUESTS).await
    }
}

fn to_rate(base: &str, quote: &str, meta: &YahooMeta) -> Option<FiatRate> {
    if meta
        .instrument_type
        .as_deref()
        .is_some_and(|t| t != "CURRENCY")
    {
        return None;
    }
    let rate = meta.regular_market_price.filter(|p| *p > 0.0)?;
    let (date, exchange_timestamp) = yahoo_time(meta.regular_market_time);
    Some(FiatRate {
        pair: format!("{base}/{quote}"),
        base: base.to_string(),
        quote: quote.to_string(),
        rate,
        bid: None,
        ask: None,
        date,
        timestamp: get_timestamp_millis(),
        exchange_timestamp,
        provider: FiatProvider::Yahoo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rate_from_meta() {
        let meta = YahooMeta {
            regular_market_price: Some(50.4395),
            regular_market_time: Some(1_794_989_536),
            instrument_type: Some("CURRENCY".to_string()),
        };
        let rate = to_rate("USD", "TRY", &meta).unwrap();
        assert_eq!(rate.pair, "USD/TRY");
        assert_eq!(rate.rate, 50.4395);
        assert_eq!(rate.date, "2026-11-18");
        assert_eq!(rate.exchange_timestamp, Some(1_794_989_536_000));
        assert!(rate.bid.is_none() && rate.ask.is_none());
    }

    #[test]
    fn rejects_non_currency_instruments() {
        let meta = YahooMeta {
            regular_market_price: Some(1.0),
            instrument_type: Some("EQUITY".to_string()),
            ..Default::default()
        };
        assert!(to_rate("EUR", "USD", &meta).is_none());
    }
}
