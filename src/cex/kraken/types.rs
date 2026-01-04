use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct KrakenDepthResponse {
    pub result: std::collections::HashMap<String, KrakenDepthData>,
}

#[derive(Debug, Deserialize)]
pub struct KrakenDepthData {
    pub asks: Vec<serde_json::Value>, // [price, quantity, timestamp]
    pub bids: Vec<serde_json::Value>, // [price, quantity, timestamp]
}
