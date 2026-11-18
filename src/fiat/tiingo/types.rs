use serde::Deserialize;

/// One element of the `GET /tiingo/fx/top?tickers=...` response. Unknown tickers are left out
/// of the array.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TiingoFxQuote {
    /// Lowercase pair such as `eurusd`. Tiingo has sent it both as `ticker` and as `index`.
    #[serde(alias = "index")]
    pub ticker: String,
    /// RFC 3339 time of the quote.
    #[serde(default)]
    pub quote_timestamp: Option<String>,
    #[serde(default)]
    pub bid_price: Option<f64>,
    #[serde(default)]
    pub ask_price: Option<f64>,
    #[serde(default)]
    pub mid_price: Option<f64>,
}
