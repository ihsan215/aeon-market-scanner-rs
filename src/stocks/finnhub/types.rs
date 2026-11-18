use serde::Deserialize;

/// `GET /api/v1/quote` response (only the fields we use; the API also returns change,
/// day high/low/open and previous close). Unknown symbols come back with every field 0.
#[derive(Debug, Default, Deserialize)]
pub struct FinnhubQuote {
    /// Current (last) price.
    #[serde(default)]
    pub c: f64,
    /// Unix seconds of the last price.
    #[serde(default)]
    pub t: i64,
}
