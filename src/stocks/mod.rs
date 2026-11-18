//! US stock prices (S&P 500 and other US-listed tickers).
//!
//! Kept apart from [`crate::cex`] and [`crate::dex`]: stocks have their own price type
//! ([`StockPrice`]) and providers, and they are not part of the crypto arbitrage scanner.
//!
//! Each provider lives in its own folder and implements [`StockTrait`]. To add one, create
//! `stocks/<provider>/`, add a [`StockProvider`] variant and implement the trait.
//!
//! | Provider | Key | Data | Batch | Free limits |
//! |---|---|---|---|---|
//! | [`Alpaca`] | yes | bid/ask (IEX), last trade, daily close | 200 tickers / request | 200 requests / min |
//! | [`Tiingo`] | yes | last price (bid/ask needs an IEX agreement) | 100 tickers / request | 50 requests / hour, 500 symbols / month |
//! | [`Finnhub`] | yes | last price | no | 60 requests / min |
//! | `YahooStocks` (feature `unofficial-yahoo`) | no | last price | no | unofficial, may rate limit |

pub mod alpaca;
pub mod finnhub;
pub mod sp500;
pub mod tiingo;
mod types;
#[cfg(feature = "unofficial-yahoo")]
pub mod yahoo;

pub use alpaca::{ALPACA_KEY_ID_ENV, ALPACA_SECRET_KEY_ENV, Alpaca, AlpacaFeed};
pub use finnhub::{FINNHUB_API_KEY_ENV, Finnhub};
pub use sp500::{SP500_SYMBOLS, SP500_SYMBOLS_AS_OF};
pub use tiingo::{TIINGO_API_TOKEN_ENV, Tiingo};
pub use types::{StockPrice, StockProvider};
#[cfg(feature = "unofficial-yahoo")]
pub use yahoo::YahooStocks;

use crate::common::{MarketScannerError, find_mid_price};
use async_trait::async_trait;
use futures::StreamExt;

/// Requests in flight at once in the default [`StockTrait::get_prices`].
const DEFAULT_MAX_CONCURRENT_REQUESTS: usize = 5;

/// Default widest bid/ask spread, as a fraction of mid (`0.02` = 2%), that providers still use
/// as the price. Wider quotes are treated as stale and the last trade is used instead.
pub const DEFAULT_MAX_SPREAD: f64 = 0.02;

/// Common interface for stock price providers.
#[async_trait]
pub trait StockTrait: Send + Sync {
    fn provider(&self) -> StockProvider;
    fn provider_name(&self) -> &str;

    async fn health_check(&self) -> Result<(), MarketScannerError>;

    /// Latest price for one ticker. Accepts `AAPL`, `brk-b`, `BRK/B` (see [`normalize_stock_symbol`]).
    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError>;

    /// Latest prices for many tickers, in input order, one entry per distinct ticker.
    ///
    /// Malformed tickers, and tickers the provider does not know or has no price for, are left
    /// out, so the result can be shorter than `symbols`. Other errors (rate limits, auth,
    /// network) are returned.
    ///
    /// The default calls [`get_price`](Self::get_price) for each ticker with up to 5 requests
    /// in flight; providers with a batch endpoint override it. Mind the provider's rate limit
    /// when passing many tickers to a provider without one.
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<StockPrice>, MarketScannerError> {
        get_prices_concurrently(self, symbols, DEFAULT_MAX_CONCURRENT_REQUESTS).await
    }

    /// Latest prices for every ticker in [`SP500_SYMBOLS`].
    async fn get_sp500_prices(&self) -> Result<Vec<StockPrice>, MarketScannerError> {
        self.get_prices(SP500_SYMBOLS).await
    }
}

/// [`StockTrait::get_prices`] on top of `get_price`: dedupes tickers, keeps `limit` requests
/// in flight, preserves input order, skips `InvalidSymbol` and stops at the first other error.
pub(crate) async fn get_prices_concurrently<P: StockTrait + ?Sized>(
    provider: &P,
    symbols: &[&str],
    limit: usize,
) -> Result<Vec<StockPrice>, MarketScannerError> {
    let tickers = normalize_stock_symbols(symbols);
    let mut results = futures::stream::iter(tickers)
        .map(|ticker| async move { provider.get_price(&ticker).await })
        .buffered(limit.max(1));

    let mut prices = Vec::new();
    while let Some(result) = results.next().await {
        match result {
            Ok(price) => prices.push(price),
            Err(MarketScannerError::InvalidSymbol(_)) => {}
            Err(e) => return Err(e),
        }
    }
    Ok(prices)
}

/// Reads a non-empty credential from the environment.
pub(crate) fn credential_from_env(
    name: &str,
    provider: &str,
) -> Result<String, MarketScannerError> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "{provider} API credentials not configured: {name} is not set"
            ))
        })
}

/// Unique normalised tickers, in input order. Malformed tickers are dropped.
pub(crate) fn normalize_stock_symbols(symbols: &[&str]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    symbols
        .iter()
        .filter_map(|s| normalize_stock_symbol(s).ok())
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

/// Mid price of a usable quote: both sides positive, not crossed, spread at most
/// `max_spread` of mid. `None` otherwise.
pub(crate) fn usable_quote_mid(bid: f64, ask: f64, max_spread: f64) -> Option<f64> {
    if !(bid.is_finite() && ask.is_finite() && bid > 0.0 && ask >= bid) {
        return None;
    }
    let mid = find_mid_price(bid, ask);
    ((ask - bid) / mid <= max_spread).then_some(mid)
}

/// Standard ticker form: uppercase, class shares with a dot (`brk-b` -> `BRK.B`).
pub fn normalize_stock_symbol(symbol: &str) -> Result<String, MarketScannerError> {
    let symbol = symbol.trim().to_uppercase().replace(['-', '/'], ".");
    if symbol.is_empty() {
        return Err(MarketScannerError::InvalidSymbol(
            "Symbol cannot be empty".to_string(),
        ));
    }
    let valid = symbol.len() <= 10
        && !symbol.starts_with('.')
        && !symbol.ends_with('.')
        && symbol
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.');
    if !valid {
        return Err(MarketScannerError::InvalidSymbol(format!(
            "Invalid stock symbol: {symbol}"
        )));
    }
    Ok(symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_symbols() {
        assert_eq!(normalize_stock_symbol(" aapl ").unwrap(), "AAPL");
        assert_eq!(normalize_stock_symbol("brk-b").unwrap(), "BRK.B");
        assert_eq!(normalize_stock_symbol("BF/B").unwrap(), "BF.B");
        assert!(normalize_stock_symbol("").is_err());
        assert!(normalize_stock_symbol("AA PL").is_err());
        assert!(normalize_stock_symbol(".B").is_err());
        assert!(normalize_stock_symbol("TOOLONGSYMBOL").is_err());
    }

    /// Fake provider: `XYZ` is unknown, `FAIL` is a rate-limit error, others cost their length.
    struct FakeProvider {
        calls: std::sync::Mutex<Vec<String>>,
    }

    #[async_trait]
    impl StockTrait for FakeProvider {
        fn provider(&self) -> StockProvider {
            StockProvider::Finnhub
        }
        fn provider_name(&self) -> &str {
            "fake"
        }
        async fn health_check(&self) -> Result<(), MarketScannerError> {
            Ok(())
        }
        async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
            self.calls.lock().unwrap().push(symbol.to_string());
            match symbol {
                "XYZ" => Err(MarketScannerError::InvalidSymbol(symbol.to_string())),
                "FAIL" => Err(MarketScannerError::ApiError("rate limited".to_string())),
                _ => Ok(StockPrice {
                    symbol: symbol.to_string(),
                    price: symbol.len() as f64,
                    bid_price: None,
                    ask_price: None,
                    bid_qty: None,
                    ask_qty: None,
                    last_price: None,
                    timestamp: 0,
                    exchange_timestamp: None,
                    provider: StockProvider::Finnhub,
                }),
            }
        }
    }

    #[tokio::test]
    async fn default_get_prices_dedupes_skips_and_keeps_order() {
        let fake = FakeProvider {
            calls: Default::default(),
        };
        let prices = fake
            .get_prices(&["MSFT", "aapl", "XYZ", "AAPL", "AA PL", "BRK-B", "A"])
            .await
            .unwrap();
        let symbols: Vec<&str> = prices.iter().map(|p| p.symbol.as_str()).collect();
        assert_eq!(symbols, ["MSFT", "AAPL", "BRK.B", "A"]);
        assert_eq!(
            fake.calls.lock().unwrap().len(),
            5,
            "one call per distinct valid ticker"
        );

        assert!(matches!(
            fake.get_prices(&["AAPL", "FAIL", "MSFT"]).await,
            Err(MarketScannerError::ApiError(_))
        ));
    }

    #[test]
    fn missing_credential_is_an_error() {
        // A name no test or .env sets, so this does not race with tests that load .env.
        let err = credential_from_env("AEON_TEST_UNSET_CREDENTIAL_VAR", "Example").unwrap_err();
        assert!(err.to_string().contains("AEON_TEST_UNSET_CREDENTIAL_VAR"));
    }

    #[test]
    fn dedupes_and_drops_malformed_symbols() {
        assert_eq!(
            normalize_stock_symbols(&["AAPL", "aapl", "AA PL", "", "brk-b", "BRK.B", "MSFT"]),
            ["AAPL", "BRK.B", "MSFT"]
        );
    }

    #[test]
    fn checks_quotes() {
        assert_eq!(usable_quote_mid(99.0, 101.0, 0.02), Some(100.0));
        assert_eq!(usable_quote_mid(99.0, 101.0, 0.01), None, "2% wide");
        assert_eq!(usable_quote_mid(101.0, 99.0, 0.5), None, "crossed");
        assert_eq!(usable_quote_mid(0.0, 101.0, 0.5), None, "no bid");
        assert_eq!(usable_quote_mid(100.0, 0.0, 0.5), None, "no ask");
        assert_eq!(usable_quote_mid(100.0, 100.0, 0.0), Some(100.0), "locked");
    }

    #[test]
    fn sp500_list_is_clean() {
        assert!(SP500_SYMBOLS.len() >= 500);
        for symbol in SP500_SYMBOLS {
            assert_eq!(normalize_stock_symbol(symbol).unwrap(), *symbol);
        }
        let unique: std::collections::HashSet<_> = SP500_SYMBOLS.iter().collect();
        assert_eq!(unique.len(), SP500_SYMBOLS.len());
    }
}
