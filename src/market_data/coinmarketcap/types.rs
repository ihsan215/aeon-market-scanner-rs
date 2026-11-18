use serde::Deserialize;

/// Envelope of every `/v3/...` response.
#[derive(Debug, Deserialize)]
pub struct CmcResponse<T> {
    #[serde(default)]
    pub data: Option<T>,
    pub status: CmcStatus,
}

#[derive(Debug, Deserialize)]
pub struct CmcStatus {
    /// `0` / `"0"` on success, otherwise a 1xxx platform code or the HTTP status. Sent as a
    /// number by some endpoints and as a string by others.
    #[serde(default)]
    pub error_code: Option<serde_json::Value>,
    #[serde(default)]
    pub error_message: Option<String>,
}

impl CmcStatus {
    /// The error code as a number, `0` when absent.
    pub fn code(&self) -> u32 {
        match &self.error_code {
            Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0) as u32,
            Some(serde_json::Value::String(s)) => s.trim().parse().unwrap_or(0),
            _ => 0,
        }
    }
}

/// One asset in `/v3/cryptocurrency/quotes/latest` (only the fields we use). A symbol lookup
/// returns every asset sharing that symbol, including inactive ones.
#[derive(Debug, Deserialize)]
pub struct CmcAsset {
    pub id: u64,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub cmc_rank: Option<u32>,
    #[serde(default)]
    pub is_active: Option<u8>,
    #[serde(default)]
    pub quote: Vec<CmcQuote>,
}

/// Market quote of an asset in one currency.
#[derive(Debug, Deserialize)]
pub struct CmcQuote {
    #[serde(default)]
    pub symbol: String,
    #[serde(default)]
    pub price: Option<f64>,
    #[serde(default)]
    pub market_cap: Option<f64>,
    #[serde(default)]
    pub volume_24h: Option<f64>,
    #[serde(default)]
    pub percent_change_24h: Option<f64>,
    /// RFC 3339.
    #[serde(default)]
    pub last_updated: Option<String>,
}
