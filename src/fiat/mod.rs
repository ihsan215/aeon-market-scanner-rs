//! Fiat exchange rates (EUR/USD, GBP/USD, USD/TRY, ...).
//!
//! Kept apart from [`crate::cex`] and [`crate::dex`]: rates have their own type
//! ([`FiatRate`]) and providers, and they are not part of the crypto arbitrage scanner.
//!
//! Each provider lives in its own folder and implements [`FiatTrait`]. To add one, create
//! `fiat/<provider>/`, add a [`FiatProvider`] variant and implement the trait.
//!
//! | Provider | Coverage | Bid/ask | Updated |
//! |---|---|---|---|
//! | [`Frankfurter`] | ~30 currencies (ECB) | no | daily, ~16:00 CET |
//! | [`ExchangeRateApi`] | 160+ currencies | no | daily |
//! | [`Tcmb`] | ~20 currencies vs TRY, crosses via TRY | yes | working days, ~15:30 Istanbul |
//! | [`TiingoFiat`] (free token) | ~140 pairs | yes | real time (seconds) |
//! | `YahooFiat` (feature `unofficial-yahoo`) | practically all pairs | no | near-live (< 1 min) |
//!
//! Frankfurter, ExchangeRate-API and TCMB need no key and publish daily reference rates.
//! [`TiingoFiat`] is real time with bid/ask and needs a free Tiingo token. `YahooFiat` needs no
//! key and is near-live, but uses an undocumented endpoint, so it is opt-in; see its docs.

mod cache;
pub mod exchangerate_api;
mod fallback;
pub mod frankfurter;
pub mod tcmb;
pub mod tiingo;
mod types;
#[cfg(feature = "unofficial-yahoo")]
pub mod yahoo;

pub use exchangerate_api::ExchangeRateApi;
pub use fallback::{FIAT_FALLBACK_DEFAULT_MAX_AGE, FiatFallback};
pub use frankfurter::Frankfurter;
pub use tcmb::Tcmb;
pub use tiingo::TiingoFiat;
pub use types::{FiatProvider, FiatRate};
#[cfg(feature = "unofficial-yahoo")]
pub use yahoo::YahooFiat;

use crate::common::MarketScannerError;
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};

/// Requests in flight at once in the default [`FiatTrait::get_rates`].
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 5;

/// Common interface for fiat exchange rate providers.
#[async_trait]
pub trait FiatTrait: Send + Sync {
    fn provider(&self) -> FiatProvider;
    fn provider_name(&self) -> &str;

    async fn health_check(&self) -> Result<(), MarketScannerError>;

    /// Rate for one pair. Accepts `EUR/USD`, `EURUSD`, `eur-usd`, `EUR_USD` (see [`parse_fiat_pair`]).
    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError>;

    /// Rates for several pairs, in input order; fails if any pair fails. The default calls
    /// [`get_rate`](Self::get_rate) for each pair with up to 5 requests in flight; providers
    /// with a batch endpoint override it.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        get_rates_concurrently(self, pairs, DEFAULT_MAX_CONCURRENT_REQUESTS).await
    }
}

/// [`FiatTrait::get_rates`] on top of `get_rate`: keeps `limit` requests in flight, preserves
/// input order and stops at the first error.
#[cfg_attr(not(feature = "unofficial-yahoo"), allow(dead_code))]
pub(crate) async fn get_rates_concurrently<P: FiatTrait + ?Sized>(
    provider: &P,
    pairs: &[&str],
    limit: usize,
) -> Result<Vec<FiatRate>, MarketScannerError> {
    // Owned strings: borrowed `&str` items trip a higher-ranked lifetime error under async_trait.
    let pairs: Vec<String> = pairs.iter().map(|p| p.to_string()).collect();
    futures::stream::iter(pairs)
        .map(|pair| async move { provider.get_rate(&pair).await })
        .buffered(limit.max(1))
        .try_collect()
        .await
}

/// ISO 4217 code check: three ASCII letters, returned uppercase.
pub fn parse_currency_code(code: &str) -> Result<String, MarketScannerError> {
    let code = code.trim().to_uppercase();
    if code.len() == 3 && code.chars().all(|c| c.is_ascii_alphabetic()) {
        Ok(code)
    } else {
        Err(MarketScannerError::InvalidSymbol(format!(
            "Invalid currency code: {code:?} (expected ISO 4217, e.g. USD)"
        )))
    }
}

/// `EUR/USD`, `EURUSD`, `eur-usd`, `EUR_USD` -> (`EUR`, `USD`).
pub fn parse_fiat_pair(pair: &str) -> Result<(String, String), MarketScannerError> {
    let compact: String = pair
        .trim()
        .chars()
        .filter(|c| !matches!(c, '/' | '-' | '_' | ' '))
        .collect();
    if compact.len() != 6 || !compact.is_ascii() {
        return Err(MarketScannerError::InvalidSymbol(format!(
            "Invalid currency pair: {pair:?} (expected e.g. EUR/USD)"
        )));
    }
    let base = parse_currency_code(&compact[..3])?;
    let quote = parse_currency_code(&compact[3..])?;
    if base == quote {
        return Err(MarketScannerError::InvalidSymbol(format!(
            "Currency pair needs two different currencies: {pair:?}"
        )));
    }
    Ok((base, quote))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pairs() {
        for input in ["EUR/USD", "EURUSD", "eur-usd", "EUR_USD", " eur / usd "] {
            assert_eq!(
                parse_fiat_pair(input).unwrap(),
                ("EUR".to_string(), "USD".to_string()),
                "{input}"
            );
        }
        for bad in [
            "", "EUR", "EUR/US", "EURUSDT", "EU1/USD", "USD/USD", "ÉUR/USD",
        ] {
            assert!(parse_fiat_pair(bad).is_err(), "{bad}");
        }
    }
}
