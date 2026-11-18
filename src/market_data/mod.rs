//! Market data providers: aggregated crypto prices (CoinMarketCap, ...).
//!
//! These are volume-weighted prices across many exchanges, with market cap and 24h stats. For
//! one exchange's live order book use [`crate::cex`] instead.
//!
//! Each provider lives in its own folder and implements [`MarketDataTrait`]. To add one, create
//! `market_data/<provider>/`, add a [`MarketDataProvider`] variant and implement the trait.

pub mod coinmarketcap;
mod types;

pub use coinmarketcap::{CMC_API_KEY_ENV, CoinMarketCap};
pub use types::{MarketDataPrice, MarketDataProvider};

use crate::common::MarketScannerError;
use async_trait::async_trait;

/// Common interface for aggregated crypto price providers.
#[async_trait]
pub trait MarketDataTrait: Send + Sync {
    fn provider(&self) -> MarketDataProvider;
    fn provider_name(&self) -> &str;

    async fn health_check(&self) -> Result<(), MarketScannerError>;

    /// Latest price for one asset symbol (`BTC`, `eth`).
    async fn get_price(&self, symbol: &str) -> Result<MarketDataPrice, MarketScannerError>;

    /// Latest prices for several symbols, in input order, one entry per distinct symbol.
    /// Unknown symbols are left out.
    async fn get_prices(
        &self,
        symbols: &[&str],
    ) -> Result<Vec<MarketDataPrice>, MarketScannerError>;
}

/// Uppercase asset symbol; letters and digits only (`btc` -> `BTC`, `1inch` -> `1INCH`).
pub fn normalize_asset_symbol(symbol: &str) -> Result<String, MarketScannerError> {
    let symbol = symbol.trim().to_uppercase();
    if symbol.is_empty() || symbol.len() > 20 || !symbol.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err(MarketScannerError::InvalidSymbol(format!(
            "Invalid asset symbol: {symbol:?} (expected e.g. BTC)"
        )));
    }
    Ok(symbol)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_symbols() {
        assert_eq!(normalize_asset_symbol(" btc ").unwrap(), "BTC");
        assert_eq!(normalize_asset_symbol("1inch").unwrap(), "1INCH");
        for bad in ["", "BTC/USD", "BT C", "BTC-USD", "ÉTH"] {
            assert!(normalize_asset_symbol(bad).is_err(), "{bad}");
        }
    }
}
