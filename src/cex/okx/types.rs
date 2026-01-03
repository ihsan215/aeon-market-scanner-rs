use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct OkxTickerResponse {
    pub code: String,
    pub msg: String,
    pub data: Vec<OkxTickerData>,
}

#[derive(Debug, Deserialize)]
pub struct OkxTickerData {
    #[serde(rename = "instId")]
    pub inst_id: String,
    #[serde(rename = "askPx")]
    pub ask_px: String,
    #[serde(rename = "askSz")]
    pub ask_sz: String,
    #[serde(rename = "bidPx")]
    pub bid_px: String,
    #[serde(rename = "bidSz")]
    pub bid_sz: String,
}
