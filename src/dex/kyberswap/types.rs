use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberSwapRoutesResponse {
    pub code: i32,
    pub message: Option<String>,
    pub data: Option<KyberSwapRoutesData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KyberSwapRoutesData {
    #[serde(rename = "routeSummary")]
    pub route_summary: RouteSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteSummary {
    #[serde(rename = "tokenIn")]
    pub token_in: String,
    #[serde(rename = "tokenOut")]
    pub token_out: String,
    #[serde(rename = "amountIn")]
    pub amount_in: String,
    #[serde(rename = "amountOut")]
    pub amount_out: String,
}
