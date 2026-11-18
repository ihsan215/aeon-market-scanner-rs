//! [`FiatFallback`]: try several fiat providers in order, preferring live rates.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use futures::StreamExt;

use crate::common::{MarketScannerError, get_timestamp_millis};
use crate::fiat::{
    ExchangeRateApi, FiatProvider, FiatRate, FiatTrait, Frankfurter, Tcmb, TiingoFiat,
    parse_fiat_pair,
};
use crate::stocks::fallback::{combine_errors, is_fresh};

/// Default for [`FiatFallback::with_max_age`].
pub const FIAT_FALLBACK_DEFAULT_MAX_AGE: Duration = Duration::from_secs(15 * 60);

/// Requests in flight when a provider's batch call failed and pairs are retried one by one.
const MAX_CONCURRENT_RETRIES: usize = 5;

/// Chains fiat providers: asks them in order and returns, per pair, the first rate that is
/// **fresh** (its `exchange_timestamp` is at most [`max_age`](Self::with_max_age) old).
///
/// - A provider that errors, does not cover a pair, or only has a stale rate for it does not
///   stop the search; the next provider is asked for the pairs still missing. If a provider's
///   batch call fails as a whole (e.g. one pair it does not cover), its pairs are retried one
///   by one so the others still count.
/// - Daily reference sources (Frankfurter, TCMB) report no quote time, so with an age limit
///   they only serve as a fallback. If no provider has a fresh rate, the first stale rate found
///   (in provider order) is returned.
/// - Each returned [`FiatRate::provider`] tells you which provider it came from.
///
/// ```no_run
/// # async fn run() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
/// use aeon_market_scanner_rs::{ExchangeRateApi, FiatFallback, FiatTrait, Tcmb, TiingoFiat};
///
/// let fx = FiatFallback::new(TiingoFiat::from_env()?)
///     .then(ExchangeRateApi::new())
///     .then(Tcmb::new());
/// let eur_usd = fx.get_rate("EUR/USD").await?;
/// let rates = fx.get_rates(&["USD/TRY", "USD/ARS"]).await?; // USD/ARS falls back
/// # Ok(()) }
/// ```
pub struct FiatFallback {
    providers: Vec<Box<dyn FiatTrait>>,
    max_age: Option<Duration>,
}

impl FiatFallback {
    /// Starts a chain with its highest-priority provider.
    pub fn new(primary: impl FiatTrait + 'static) -> Self {
        Self {
            providers: vec![Box::new(primary)],
            max_age: Some(FIAT_FALLBACK_DEFAULT_MAX_AGE),
        }
    }

    /// Appends a lower-priority provider.
    pub fn then(mut self, provider: impl FiatTrait + 'static) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Default chain: Tiingo FX (when `TIINGO_API_TOKEN` is set), `YahooFiat` (when the
    /// `unofficial-yahoo` feature is on), then ExchangeRate-API, Frankfurter and TCMB, which
    /// need no key.
    pub fn from_env() -> Self {
        let mut providers: Vec<Box<dyn FiatTrait>> = Vec::new();
        if let Ok(p) = TiingoFiat::from_env() {
            providers.push(Box::new(p));
        }
        #[cfg(feature = "unofficial-yahoo")]
        providers.push(Box::new(crate::fiat::YahooFiat::new()));
        providers.push(Box::new(ExchangeRateApi::new()));
        providers.push(Box::new(Frankfurter::new()));
        providers.push(Box::new(Tcmb::new()));
        Self {
            providers,
            max_age: Some(FIAT_FALLBACK_DEFAULT_MAX_AGE),
        }
    }

    /// Oldest rate still treated as live (default 15 minutes). `None` accepts any rate, so
    /// later providers are only asked on errors and uncovered pairs.
    pub fn with_max_age(mut self, max_age: Option<Duration>) -> Self {
        self.max_age = max_age;
        self
    }

    /// Providers in priority order.
    pub fn providers(&self) -> impl Iterator<Item = FiatProvider> + '_ {
        self.providers.iter().map(|p| p.provider())
    }

    fn is_fresh(&self, rate: &FiatRate, now: u64) -> bool {
        is_fresh(rate.exchange_timestamp, self.max_age, now)
    }
}

/// Rates for `pairs` from one provider: the batch call, or pair by pair if the batch failed.
/// Returns the rates it got and, if nothing came back, the error.
async fn rates_from(
    provider: &dyn FiatTrait,
    pairs: &[String],
) -> (Vec<FiatRate>, Option<MarketScannerError>) {
    let refs: Vec<&str> = pairs.iter().map(String::as_str).collect();
    let batch_error = match provider.get_rates(&refs).await {
        Ok(rates) => return (rates, None),
        Err(e) => e,
    };
    if pairs.len() == 1 {
        return (Vec::new(), Some(batch_error));
    }

    let mut results = futures::stream::iter(pairs.to_vec())
        .map(|pair| async move { provider.get_rate(&pair).await })
        .buffered(MAX_CONCURRENT_RETRIES);
    let mut rates = Vec::new();
    let mut last_error = None;
    while let Some(result) = results.next().await {
        match result {
            Ok(rate) => rates.push(rate),
            Err(e) => last_error = Some(e),
        }
    }
    let error = rates.is_empty().then(|| last_error.unwrap_or(batch_error));
    (rates, error)
}

#[async_trait]
impl FiatTrait for FiatFallback {
    /// The primary (first) provider.
    fn provider(&self) -> FiatProvider {
        self.providers[0].provider()
    }

    fn provider_name(&self) -> &str {
        "Fallback"
    }

    /// Ok when at least one provider is healthy.
    async fn health_check(&self) -> Result<(), MarketScannerError> {
        let mut errors = Vec::new();
        for p in &self.providers {
            match p.health_check().await {
                Ok(()) => return Ok(()),
                Err(e) => errors.push((p.provider_name().to_string(), e)),
            }
        }
        Err(combine_errors("a health check", errors))
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let (base, quote) = parse_fiat_pair(pair)?;
        let pair = format!("{base}/{quote}");
        let mut stale = None;
        let mut errors = Vec::new();
        for p in &self.providers {
            match p.get_rate(&pair).await {
                Ok(rate) if self.is_fresh(&rate, get_timestamp_millis()) => return Ok(rate),
                Ok(rate) => {
                    stale.get_or_insert(rate);
                }
                Err(e) => errors.push((p.provider_name().to_string(), e)),
            }
        }
        stale.ok_or_else(|| combine_errors(&pair, errors))
    }

    /// Asks each provider only for the pairs still without a fresh rate. Results keep input
    /// order (duplicates included); fails if some pair is served by no provider.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        let requested = pairs
            .iter()
            .map(|pair| parse_fiat_pair(pair).map(|(b, q)| format!("{b}/{q}")))
            .collect::<Result<Vec<_>, _>>()?;
        let mut unique = requested.clone();
        unique.sort();
        unique.dedup();

        let mut fresh: HashMap<String, FiatRate> = HashMap::new();
        let mut stale: HashMap<String, FiatRate> = HashMap::new();
        let mut errors = Vec::new();

        for p in &self.providers {
            let remaining: Vec<String> = unique
                .iter()
                .filter(|pair| !fresh.contains_key(*pair))
                .cloned()
                .collect();
            if remaining.is_empty() {
                break;
            }
            let (rates, error) = rates_from(p.as_ref(), &remaining).await;
            if let Some(e) = error {
                errors.push((p.provider_name().to_string(), e));
            }
            let now = get_timestamp_millis();
            for rate in rates {
                if self.is_fresh(&rate, now) {
                    stale.remove(&rate.pair);
                    fresh.insert(rate.pair.clone(), rate);
                } else {
                    stale.entry(rate.pair.clone()).or_insert(rate);
                }
            }
        }

        let missing: Vec<&str> = unique
            .iter()
            .filter(|pair| !fresh.contains_key(*pair) && !stale.contains_key(*pair))
            .map(String::as_str)
            .collect();
        if !missing.is_empty() {
            return Err(combine_errors(&missing.join(", "), errors));
        }
        Ok(requested
            .iter()
            .filter_map(|pair| fresh.get(pair).or_else(|| stale.get(pair)).cloned())
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    /// Fake provider: `rates` maps pair -> age in ms (`None` = unknown age); pairs not in the
    /// map are uncovered, and its batch call fails if any pair is uncovered (like the real
    /// daily providers). `fail` makes every call error.
    struct Fake {
        provider: FiatProvider,
        rates: HashMap<&'static str, Option<u64>>,
        fail: bool,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl Fake {
        fn new(provider: FiatProvider, rates: &[(&'static str, Option<u64>)]) -> Self {
            Self {
                provider,
                rates: rates.iter().copied().collect(),
                fail: false,
                asked: Arc::default(),
            }
        }
        fn failing(provider: FiatProvider) -> Self {
            Self {
                fail: true,
                ..Self::new(provider, &[])
            }
        }
    }

    #[async_trait]
    impl FiatTrait for Fake {
        fn provider(&self) -> FiatProvider {
            self.provider
        }
        fn provider_name(&self) -> &str {
            "fake"
        }
        async fn health_check(&self) -> Result<(), MarketScannerError> {
            Ok(())
        }
        async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
            self.asked.lock().unwrap().push(pair.to_string());
            if self.fail {
                return Err(MarketScannerError::ApiError("down".into()));
            }
            let age = *self
                .rates
                .get(pair)
                .ok_or_else(|| MarketScannerError::InvalidSymbol(pair.into()))?;
            let now = get_timestamp_millis();
            let (base, quote) = pair.split_once('/').unwrap();
            Ok(FiatRate {
                pair: pair.to_string(),
                base: base.to_string(),
                quote: quote.to_string(),
                rate: 1.0,
                bid: None,
                ask: None,
                date: String::new(),
                timestamp: now,
                exchange_timestamp: age.map(|a| now - a),
                provider: self.provider,
            })
        }
    }

    const FRESH: Option<u64> = Some(1_000);

    #[tokio::test]
    async fn single_prefers_live_then_daily() {
        let fx = FiatFallback::new(Fake::new(FiatProvider::Tiingo, &[("EUR/USD", FRESH)]))
            .then(Fake::failing(FiatProvider::Yahoo))
            .then(Fake::new(
                FiatProvider::ExchangeRateApi,
                &[("EUR/USD", None), ("USD/ARS", None)],
            ));

        assert_eq!(
            fx.get_rate("eur-usd").await.unwrap().provider,
            FiatProvider::Tiingo
        );
        // Tiingo does not cover it, Yahoo is down, the daily source has it.
        assert_eq!(
            fx.get_rate("USD/ARS").await.unwrap().provider,
            FiatProvider::ExchangeRateApi
        );
        assert!(fx.get_rate("XXX/YYY").await.is_err());
    }

    #[tokio::test]
    async fn batch_falls_back_per_pair_and_keeps_order() {
        let tiingo = Fake::new(
            FiatProvider::Tiingo,
            &[("EUR/USD", FRESH), ("USD/TRY", FRESH)],
        );
        let er_api = Fake::new(
            FiatProvider::ExchangeRateApi,
            &[("EUR/USD", None), ("USD/ARS", None), ("USD/TRY", None)],
        );
        let er_asked = Arc::clone(&er_api.asked);
        let fx = FiatFallback::new(tiingo).then(er_api);

        let rates = fx
            .get_rates(&["USD/TRY", "USD/ARS", "EUR/USD", "usd-try"])
            .await
            .unwrap();
        let got: Vec<(&str, FiatProvider)> = rates
            .iter()
            .map(|r| (r.pair.as_str(), r.provider))
            .collect();
        assert_eq!(
            got,
            [
                ("USD/TRY", FiatProvider::Tiingo),
                ("USD/ARS", FiatProvider::ExchangeRateApi),
                ("EUR/USD", FiatProvider::Tiingo),
                ("USD/TRY", FiatProvider::Tiingo),
            ]
        );
        // Only the pair Tiingo could not serve went to the next provider.
        assert_eq!(*er_asked.lock().unwrap(), ["USD/ARS"]);
    }

    #[tokio::test]
    async fn batch_errors_when_a_pair_is_served_by_nobody() {
        let fx = FiatFallback::new(Fake::new(FiatProvider::Tiingo, &[("EUR/USD", FRESH)]))
            .then(Fake::new(FiatProvider::Tcmb, &[]));
        let err = fx.get_rates(&["EUR/USD", "XXX/YYY"]).await.unwrap_err();
        assert!(matches!(err, MarketScannerError::InvalidSymbol(_)));
        assert!(err.to_string().contains("XXX/YYY"));
    }
}
