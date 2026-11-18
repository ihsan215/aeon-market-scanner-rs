use serde::Deserialize;

/// One element of the `GET /iex/?tickers=...` response.
///
/// Fields marked "IEX entitlement" are `null` unless the account has an IEX market data
/// agreement (required for the full IEX feed since 2025-02-01).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiingoIexQuote {
    pub ticker: String,
    /// When Tiingo last refreshed this record (RFC 3339).
    #[serde(default)]
    pub timestamp: Option<String>,
    /// Last bid/ask time (IEX entitlement).
    #[serde(default)]
    pub quote_timestamp: Option<String>,
    /// Last trade time (IEX entitlement).
    #[serde(default)]
    pub last_sale_timestamp: Option<String>,
    /// Last trade on IEX (IEX entitlement).
    #[serde(default)]
    pub last: Option<f64>,
    /// Tiingo-computed last price (last trade or mid).
    #[serde(default)]
    pub tngo_last: Option<f64>,
    /// IEX entitlement.
    #[serde(default)]
    pub bid_price: Option<f64>,
    /// IEX entitlement.
    #[serde(default)]
    pub bid_size: Option<f64>,
    /// IEX entitlement.
    #[serde(default)]
    pub ask_price: Option<f64>,
    /// IEX entitlement.
    #[serde(default)]
    pub ask_size: Option<f64>,
}
