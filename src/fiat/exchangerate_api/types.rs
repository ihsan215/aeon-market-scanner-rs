use std::collections::HashMap;

use serde::Deserialize;

/// `GET /v6/latest/{BASE}` response. Errors also come back with HTTP 200 and
/// `"result": "error"`, `"error-type": "..."`.
#[derive(Debug, Deserialize)]
pub struct ErApiLatestResponse {
    pub result: String,
    #[serde(default, rename = "error-type")]
    pub error_type: Option<String>,
    #[serde(default)]
    pub base_code: Option<String>,
    #[serde(default)]
    pub time_last_update_unix: Option<i64>,
    #[serde(default)]
    pub rates: HashMap<String, f64>,
    /// Local time (ms since epoch) this response was downloaded; set after parsing.
    #[serde(skip)]
    pub fetched_at: u64,
}
