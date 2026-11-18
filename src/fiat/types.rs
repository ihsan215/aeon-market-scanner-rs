use serde::{Deserialize, Serialize};

/// Data providers for fiat exchange rates.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum FiatProvider {
    /// ECB reference rates via frankfurter.dev.
    Frankfurter,
    /// ExchangeRate-API open access endpoint (open.er-api.com).
    ExchangeRateApi,
    /// Central Bank of the Republic of Türkiye indicative rates (tcmb.gov.tr).
    Tcmb,
    /// Tiingo FX real-time top-of-book quotes (api.tiingo.com).
    Tiingo,
    /// Yahoo Finance, unofficial endpoint (feature `unofficial-yahoo`).
    Yahoo,
}

/// Exchange rate for a fiat currency pair: `1 base = rate quote`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiatRate {
    /// Pair in `BASE/QUOTE` form, e.g. `EUR/USD`.
    pub pair: String,
    pub base: String,
    pub quote: String,
    /// Mid rate. When the provider gives buying/selling prices this is their midpoint.
    pub rate: f64,
    /// Buying price, for providers that publish one (TCMB). `None` for mid-only sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid: Option<f64>,
    /// Selling price, for providers that publish one (TCMB). `None` for mid-only sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask: Option<f64>,
    /// Reference date of the rate (`YYYY-MM-DD`). The free providers publish once per day,
    /// so this tells you how old the rate is.
    pub date: String,
    /// Local receive time (ms since epoch): when this library parsed the response.
    pub timestamp: u64,
    /// Last update time reported by the provider (ms since epoch), when it gives one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_timestamp: Option<u64>,
    pub provider: FiatProvider,
}
