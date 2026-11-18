use std::collections::HashMap;

use serde::Deserialize;

/// `GET /v1/latest` response.
#[derive(Debug, Deserialize)]
pub struct FrankfurterLatestResponse {
    pub base: String,
    pub date: String,
    #[serde(default)]
    pub rates: HashMap<String, f64>,
}
