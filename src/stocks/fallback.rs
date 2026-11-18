//! [`StockFallback`]: try several stock providers in order, preferring live prices.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;

use crate::common::{MarketScannerError, get_timestamp_millis};
use crate::stocks::{
    Alpaca, Finnhub, StockPrice, StockProvider, StockTrait, Tiingo, normalize_stock_symbol,
    normalize_stock_symbols,
};

/// Default for [`StockFallback::with_max_age`].
pub const STOCK_FALLBACK_DEFAULT_MAX_AGE: Duration = Duration::from_secs(15 * 60);

/// Chains stock providers: asks them in order and returns, per ticker, the first price that is
/// **fresh** (its `exchange_timestamp` is at most [`max_age`](Self::with_max_age) old).
///
/// - A provider that errors (network, rate limit, auth), does not know a ticker, or only has a
///   stale price for it does not stop the search; the next provider is asked for the tickers
///   still missing.
/// - If no provider has a fresh price, the first stale price found (in provider order) is
///   returned, so outside market hours you still get the last known price.
/// - The age limit only applies during the US regular session (Mon–Fri 09:30–16:00 New York
///   time; exchange holidays are not known). Outside it every provider's price is old, so
///   later providers are only asked on errors and missing tickers, which avoids spending their
///   quota for nothing.
/// - Each returned [`StockPrice::provider`] tells you which provider it came from.
///
/// It implements [`StockTrait`], so `get_price`, `get_prices` and `get_sp500_prices` all work:
///
/// ```no_run
/// # async fn run() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
/// use aeon_market_scanner_rs::{Alpaca, StockFallback, StockTrait, Tiingo};
///
/// let stocks = StockFallback::new(Alpaca::from_env()?).then(Tiingo::from_env()?);
/// let aapl = stocks.get_price("AAPL").await?;
/// let sp500 = stocks.get_sp500_prices().await?;
/// # Ok(()) }
/// ```
///
/// Cost: when the first provider's prices are stale (e.g. the market is closed), the later
/// providers are asked too. Put batch-capable providers (Alpaca, Tiingo) before per-ticker
/// ones (Finnhub, Yahoo), or use `with_max_age(None)` to fall back on errors and missing
/// tickers only.
pub struct StockFallback {
    providers: Vec<Box<dyn StockTrait>>,
    max_age: Option<Duration>,
    session_only: bool,
}

impl StockFallback {
    /// Starts a chain with its highest-priority provider.
    pub fn new(primary: impl StockTrait + 'static) -> Self {
        Self {
            providers: vec![Box::new(primary)],
            max_age: Some(STOCK_FALLBACK_DEFAULT_MAX_AGE),
            session_only: true,
        }
    }

    /// Appends a lower-priority provider.
    pub fn then(mut self, provider: impl StockTrait + 'static) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Chain of every provider whose credentials are set, in the order Alpaca, Tiingo,
    /// Finnhub, then `YahooStocks` when the `unofficial-yahoo` feature is on. Errors if none
    /// is available.
    pub fn from_env() -> Result<Self, MarketScannerError> {
        let mut providers: Vec<Box<dyn StockTrait>> = Vec::new();
        if let Ok(p) = Alpaca::from_env() {
            providers.push(Box::new(p));
        }
        if let Ok(p) = Tiingo::from_env() {
            providers.push(Box::new(p));
        }
        if let Ok(p) = Finnhub::from_env() {
            providers.push(Box::new(p));
        }
        #[cfg(feature = "unofficial-yahoo")]
        providers.push(Box::new(crate::stocks::YahooStocks::new()));

        if providers.is_empty() {
            return Err(MarketScannerError::ApiError(
                "No stock provider configured: set ALPACA_API_KEY_ID + ALPACA_API_SECRET_KEY, \
                 TIINGO_API_TOKEN or FINNHUB_API_KEY"
                    .to_string(),
            ));
        }
        Ok(Self {
            providers,
            max_age: Some(STOCK_FALLBACK_DEFAULT_MAX_AGE),
            session_only: true,
        })
    }

    /// Oldest price still treated as live (default 15 minutes). `None` accepts any price, so
    /// later providers are only asked on errors and missing tickers.
    pub fn with_max_age(mut self, max_age: Option<Duration>) -> Self {
        self.max_age = max_age;
        self
    }

    /// Whether the age limit applies only during the US regular session (default `true`).
    /// With `false` it always applies, so outside market hours every provider is asked.
    pub fn with_age_limit_in_session_only(mut self, session_only: bool) -> Self {
        self.session_only = session_only;
        self
    }

    /// Providers in priority order.
    pub fn providers(&self) -> impl Iterator<Item = StockProvider> + '_ {
        self.providers.iter().map(|p| p.provider())
    }

    fn is_fresh(&self, price: &StockPrice, now: u64) -> bool {
        (self.session_only && !us_regular_session_open(now))
            || is_fresh(price.exchange_timestamp, self.max_age, now)
    }
}

/// `true` during the US regular trading session, Mon–Fri 09:30–16:00 New York time. US
/// daylight saving (second Sunday of March to first Sunday of November, at 02:00 local) is
/// handled; exchange holidays and early closes are not.
pub(crate) fn us_regular_session_open(now_ms: u64) -> bool {
    use chrono::{Datelike, NaiveDate, Timelike, Weekday};

    let Some(utc) = i64::try_from(now_ms)
        .ok()
        .and_then(chrono::DateTime::from_timestamp_millis)
    else {
        return false;
    };
    let year = utc.year();
    let nth_sunday =
        |month: u32, n: u8| NaiveDate::from_weekday_of_month_opt(year, month, Weekday::Sun, n);
    // DST starts at 02:00 EST (07:00 UTC) and ends at 02:00 EDT (06:00 UTC).
    let dst = match (nth_sunday(3, 2), nth_sunday(11, 1)) {
        (Some(start), Some(end)) => {
            let start = start.and_hms_opt(7, 0, 0).map(|t| t.and_utc());
            let end = end.and_hms_opt(6, 0, 0).map(|t| t.and_utc());
            matches!((start, end), (Some(s), Some(e)) if utc >= s && utc < e)
        }
        _ => false,
    };
    let offset_hours = if dst { -4 } else { -5 };
    let local = utc.naive_utc() + chrono::Duration::hours(offset_hours);

    let minutes = local.hour() * 60 + local.minute();
    !matches!(local.weekday(), Weekday::Sat | Weekday::Sun)
        && (9 * 60 + 30..16 * 60).contains(&minutes)
}

/// `true` when there is no age limit, or `exchange_timestamp` is known and recent enough.
pub(crate) fn is_fresh(
    exchange_timestamp: Option<u64>,
    max_age: Option<Duration>,
    now: u64,
) -> bool {
    match max_age {
        None => true,
        Some(max_age) => exchange_timestamp.is_some_and(|t| {
            now.saturating_sub(t) <= u64::try_from(max_age.as_millis()).unwrap_or(u64::MAX)
        }),
    }
}

/// Combines per-provider errors into one: `InvalidSymbol` when every provider said so.
pub(crate) fn combine_errors(
    subject: &str,
    errors: Vec<(String, MarketScannerError)>,
) -> MarketScannerError {
    let all_invalid = errors
        .iter()
        .all(|(_, e)| matches!(e, MarketScannerError::InvalidSymbol(_)));
    let details = errors
        .iter()
        .map(|(name, e)| format!("{name}: {e}"))
        .collect::<Vec<_>>()
        .join("; ");
    let message = format!("no provider could serve {subject} ({details})");
    if all_invalid {
        MarketScannerError::InvalidSymbol(message)
    } else {
        MarketScannerError::ApiError(message)
    }
}

#[async_trait]
impl StockTrait for StockFallback {
    /// The primary (first) provider.
    fn provider(&self) -> StockProvider {
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

    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
        let symbol = normalize_stock_symbol(symbol)?;
        let mut stale = None;
        let mut errors = Vec::new();
        for p in &self.providers {
            match p.get_price(&symbol).await {
                Ok(price) if self.is_fresh(&price, get_timestamp_millis()) => return Ok(price),
                Ok(price) => {
                    stale.get_or_insert(price);
                }
                Err(e) => errors.push((p.provider_name().to_string(), e)),
            }
        }
        stale.ok_or_else(|| combine_errors(&symbol, errors))
    }

    /// Asks each provider only for the tickers still without a fresh price. Tickers no
    /// provider has are left out; fails only if every provider failed.
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<StockPrice>, MarketScannerError> {
        let tickers = normalize_stock_symbols(symbols);
        let mut fresh: HashMap<String, StockPrice> = HashMap::new();
        let mut stale: HashMap<String, StockPrice> = HashMap::new();
        let mut errors = Vec::new();

        for p in &self.providers {
            let remaining: Vec<&str> = tickers
                .iter()
                .filter(|t| !fresh.contains_key(*t))
                .map(String::as_str)
                .collect();
            if remaining.is_empty() {
                break;
            }
            match p.get_prices(&remaining).await {
                Ok(prices) => {
                    let now = get_timestamp_millis();
                    for price in prices {
                        if self.is_fresh(&price, now) {
                            stale.remove(&price.symbol);
                            fresh.insert(price.symbol.clone(), price);
                        } else {
                            stale.entry(price.symbol.clone()).or_insert(price);
                        }
                    }
                }
                Err(e) => errors.push((p.provider_name().to_string(), e)),
            }
        }

        if fresh.is_empty() && stale.is_empty() && errors.len() == self.providers.len() {
            return Err(combine_errors("any of the tickers", errors));
        }
        Ok(tickers
            .iter()
            .filter_map(|t| fresh.remove(t).or_else(|| stale.remove(t)))
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    const NOW_GUARD_MS: u64 = 60_000;

    /// Fake provider: `prices` maps ticker -> age in ms (`None` = unknown age); tickers not in
    /// the map are unknown. `fail` makes every call error. Records the tickers it was asked for.
    struct Fake {
        provider: StockProvider,
        prices: HashMap<&'static str, Option<u64>>,
        fail: bool,
        asked: Arc<Mutex<Vec<String>>>,
    }

    impl Fake {
        fn new(provider: StockProvider, prices: &[(&'static str, Option<u64>)]) -> Self {
            Self {
                provider,
                prices: prices.iter().copied().collect(),
                fail: false,
                asked: Arc::default(),
            }
        }
        fn failing(provider: StockProvider) -> Self {
            Self {
                fail: true,
                ..Self::new(provider, &[])
            }
        }
    }

    #[async_trait]
    impl StockTrait for Fake {
        fn provider(&self) -> StockProvider {
            self.provider
        }
        fn provider_name(&self) -> &str {
            "fake"
        }
        async fn health_check(&self) -> Result<(), MarketScannerError> {
            if self.fail {
                Err(MarketScannerError::ApiError("down".into()))
            } else {
                Ok(())
            }
        }
        async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
            self.asked.lock().unwrap().push(symbol.to_string());
            if self.fail {
                return Err(MarketScannerError::ApiError("rate limited".into()));
            }
            let age = *self
                .prices
                .get(symbol)
                .ok_or_else(|| MarketScannerError::InvalidSymbol(symbol.into()))?;
            let now = get_timestamp_millis();
            Ok(StockPrice {
                symbol: symbol.to_string(),
                price: 1.0,
                bid_price: None,
                ask_price: None,
                bid_qty: None,
                ask_qty: None,
                last_price: None,
                timestamp: now,
                exchange_timestamp: age.map(|a| now - a),
                provider: self.provider,
            })
        }
    }

    const FRESH: Option<u64> = Some(1_000);
    const STALE: Option<u64> = Some(60 * 60 * 1000);

    #[tokio::test]
    async fn single_prefers_fresh_then_stale_then_errors() {
        let chain = StockFallback::new(Fake::new(StockProvider::Alpaca, &[("AAPL", STALE)]))
            .then(Fake::failing(StockProvider::Tiingo))
            .then(Fake::new(
                StockProvider::Finnhub,
                &[("AAPL", FRESH), ("MSFT", STALE)],
            ))
            .with_age_limit_in_session_only(false);

        // Alpaca stale, Tiingo errors, Finnhub fresh -> Finnhub.
        assert_eq!(
            chain.get_price("aapl").await.unwrap().provider,
            StockProvider::Finnhub
        );
        // Only stale prices anywhere -> the first stale one (Finnhub's MSFT).
        let msft = chain.get_price("MSFT").await.unwrap();
        assert_eq!(msft.provider, StockProvider::Finnhub);
        // Unknown everywhere and one provider errored -> ApiError listing them.
        assert!(matches!(
            chain.get_price("XYZ").await,
            Err(MarketScannerError::ApiError(_))
        ));
    }

    #[tokio::test]
    async fn single_without_max_age_takes_first_answer() {
        let chain = StockFallback::new(Fake::new(StockProvider::Alpaca, &[("AAPL", STALE)]))
            .then(Fake::new(StockProvider::Finnhub, &[("AAPL", FRESH)]))
            .with_max_age(None);
        assert_eq!(
            chain.get_price("AAPL").await.unwrap().provider,
            StockProvider::Alpaca
        );
    }

    #[tokio::test]
    async fn unknown_everywhere_is_invalid_symbol() {
        let chain = StockFallback::new(Fake::new(StockProvider::Alpaca, &[]))
            .then(Fake::new(StockProvider::Finnhub, &[]));
        assert!(matches!(
            chain.get_price("XYZ").await,
            Err(MarketScannerError::InvalidSymbol(_))
        ));
    }

    #[tokio::test]
    async fn batch_asks_later_providers_only_for_what_is_missing() {
        let alpaca = Fake::new(
            StockProvider::Alpaca,
            &[("AAPL", FRESH), ("MSFT", STALE), ("NVDA", None)],
        );
        let finnhub = Fake::new(
            StockProvider::Finnhub,
            &[
                ("MSFT", FRESH),
                ("NVDA", STALE),
                ("AAPL", FRESH),
                ("SPY", FRESH),
            ],
        );
        let finnhub_asked = Arc::clone(&finnhub.asked);
        let chain = StockFallback::new(alpaca)
            .then(Fake::failing(StockProvider::Tiingo))
            .then(finnhub)
            .with_age_limit_in_session_only(false);

        let prices = chain
            .get_prices(&["AAPL", "MSFT", "NVDA", "SPY", "XYZ", "aapl"])
            .await
            .unwrap();
        let got: Vec<(&str, StockProvider)> = prices
            .iter()
            .map(|p| (p.symbol.as_str(), p.provider))
            .collect();
        assert_eq!(
            got,
            [
                ("AAPL", StockProvider::Alpaca),
                ("MSFT", StockProvider::Finnhub),
                ("NVDA", StockProvider::Alpaca), // stale everywhere: first stale wins
                ("SPY", StockProvider::Finnhub),
            ]
        );

        // AAPL was fresh at Alpaca, so Finnhub was not asked for it.
        assert_eq!(
            *finnhub_asked.lock().unwrap(),
            ["MSFT", "NVDA", "SPY", "XYZ"]
        );
    }

    #[tokio::test]
    async fn batch_fails_only_when_every_provider_fails() {
        let chain = StockFallback::new(Fake::failing(StockProvider::Alpaca))
            .then(Fake::failing(StockProvider::Finnhub));
        assert!(chain.get_prices(&["AAPL"]).await.is_err());
        assert!(chain.health_check().await.is_err());

        let chain = StockFallback::new(Fake::failing(StockProvider::Alpaca))
            .then(Fake::new(StockProvider::Finnhub, &[]));
        assert!(chain.get_prices(&["AAPL"]).await.unwrap().is_empty());
        assert!(chain.health_check().await.is_ok());
    }

    fn ms(rfc3339: &str) -> u64 {
        crate::common::utils::iso8601_ms(rfc3339).unwrap()
    }

    #[tokio::test]
    async fn outside_the_session_stale_prices_do_not_escalate() {
        let finnhub = Fake::new(StockProvider::Finnhub, &[("AAPL", FRESH)]);
        let asked = Arc::clone(&finnhub.asked);
        let chain =
            StockFallback::new(Fake::new(StockProvider::Alpaca, &[("AAPL", STALE)])).then(finnhub);
        let now = get_timestamp_millis();
        let price = chain.get_price("AAPL").await.unwrap();
        if us_regular_session_open(now) {
            assert_eq!(price.provider, StockProvider::Finnhub);
        } else {
            assert_eq!(price.provider, StockProvider::Alpaca);
            assert!(
                asked.lock().unwrap().is_empty(),
                "closed market: no escalation"
            );
        }
    }

    #[test]
    fn us_session_hours() {
        // Winter (EST, UTC-5): 09:30-16:00 New York = 14:30-21:00 UTC.
        assert!(!us_regular_session_open(ms("2026-11-18T14:29:59Z")));
        assert!(us_regular_session_open(ms("2026-11-18T14:30:00Z")));
        assert!(us_regular_session_open(ms("2026-11-18T20:59:59Z")));
        assert!(!us_regular_session_open(ms("2026-11-18T21:00:00Z")));
        // Summer (EDT, UTC-4): 13:30-20:00 UTC.
        assert!(us_regular_session_open(ms("2026-07-15T13:30:00Z")));
        assert!(!us_regular_session_open(ms("2026-07-15T20:00:00Z")));
        // Weekend.
        assert!(!us_regular_session_open(ms("2026-11-21T16:00:00Z")));
        // DST switches: 2026-03-08 and 2026-11-01. Monday after each uses the new offset.
        assert!(us_regular_session_open(ms("2026-03-09T13:30:00Z")));
        assert!(!us_regular_session_open(ms("2026-03-06T13:30:00Z")));
        assert!(us_regular_session_open(ms("2026-11-02T14:30:00Z")));
        assert!(!us_regular_session_open(ms("2026-10-30T20:00:00Z")));
    }

    #[test]
    fn freshness_rules() {
        let now = 10 * NOW_GUARD_MS;
        let max = Some(Duration::from_millis(NOW_GUARD_MS));
        assert!(is_fresh(Some(now - 1), max, now));
        assert!(is_fresh(Some(now - NOW_GUARD_MS), max, now));
        assert!(!is_fresh(Some(now - NOW_GUARD_MS - 1), max, now));
        assert!(!is_fresh(None, max, now), "unknown age is not live");
        assert!(is_fresh(None, None, now), "no limit accepts anything");
        assert!(
            is_fresh(Some(now + 5), max, now),
            "clock skew into the future"
        );
    }
}
