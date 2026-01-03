use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BybitTickerData {
    pub symbol: String,
    #[serde(rename = "bid1Price")]
    pub bid1_price: String,
    #[serde(rename = "bid1Size")]
    pub bid1_size: String,
    #[serde(rename = "ask1Price")]
    pub ask1_price: String,
    #[serde(rename = "ask1Size")]
    pub ask1_size: String,
}
