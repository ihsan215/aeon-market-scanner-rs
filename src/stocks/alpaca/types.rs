use std::collections::HashMap;

use serde::Deserialize;

/// `GET /v2/stocks/snapshots` response: snapshot per symbol. Unknown symbols are left out.
pub type AlpacaSnapshotsResponse = HashMap<String, AlpacaSnapshot>;

/// One snapshot (only the parts we use; the API also returns minute and previous-day bars).
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlpacaSnapshot {
    #[serde(default)]
    pub latest_quote: Option<AlpacaQuote>,
    #[serde(default)]
    pub latest_trade: Option<AlpacaTrade>,
    #[serde(default)]
    pub daily_bar: Option<AlpacaBar>,
}

/// Latest quote. Prices of 0 mean there is no active bid / ask.
#[derive(Debug, Default, Deserialize)]
pub struct AlpacaQuote {
    #[serde(default)]
    pub bp: f64,
    #[serde(default)]
    pub bs: f64,
    #[serde(default)]
    pub ap: f64,
    #[serde(default, rename = "as")]
    pub ask_size: f64,
    /// RFC 3339 quote time.
    #[serde(default)]
    pub t: Option<String>,
}

/// Latest trade.
#[derive(Debug, Default, Deserialize)]
pub struct AlpacaTrade {
    /// Trade price.
    #[serde(default)]
    pub p: f64,
    /// RFC 3339 trade time.
    #[serde(default)]
    pub t: Option<String>,
}

/// Daily bar (only the close is used, as a last-resort price).
#[derive(Debug, Default, Deserialize)]
pub struct AlpacaBar {
    /// Close price.
    #[serde(default)]
    pub c: f64,
    /// RFC 3339 bar start time.
    #[serde(default)]
    pub t: Option<String>,
}
