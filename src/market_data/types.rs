use serde::{Deserialize, Serialize};

/// Providers of aggregated crypto prices.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum MarketDataProvider {
    CoinMarketCap,
}

/// Aggregated price of a cryptocurrency, as published by a market data provider. It is a
/// volume-weighted figure across many exchanges, not one exchange's order book.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketDataPrice {
    /// Asset symbol, e.g. `BTC`.
    pub symbol: String,
    /// Asset name, e.g. `Bitcoin`.
    pub name: String,
    /// The provider's own id for the asset (CoinMarketCap id `1` is Bitcoin). Symbols are not
    /// unique across assets; the id is.
    pub provider_id: String,
    /// Currency the price is quoted in, e.g. `USD`.
    pub quote: String,
    pub price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub market_cap: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume_24h: Option<f64>,
    /// Percent change over 24 hours (`-1.5` = -1.5%).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub percent_change_24h: Option<f64>,
    /// Rank by market cap, when the provider ranks the asset.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rank: Option<u32>,
    /// Local receive time (ms since epoch): when this library parsed the response.
    pub timestamp: u64,
    /// When the provider last updated this price (ms since epoch).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_timestamp: Option<u64>,
    pub provider: MarketDataProvider,
}
