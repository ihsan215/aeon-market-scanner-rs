use serde::Deserialize;

/// Upbit orderbook response format
#[derive(Debug, Deserialize)]
pub struct UpbitOrderBookResponse {
    #[serde(rename = "orderbook_units")]
    pub orderbook_units: Vec<UpbitOrderBookUnit>,
}

/// Upbit orderbook unit - contains bid and ask for a price level
#[derive(Debug, Deserialize)]
pub struct UpbitOrderBookUnit {
    #[serde(rename = "bid_price")]
    pub bid_price: f64,
    #[serde(rename = "bid_size")]
    pub bid_size: f64,
    #[serde(rename = "ask_price")]
    pub ask_price: f64,
    #[serde(rename = "ask_size")]
    pub ask_size: f64,
}
