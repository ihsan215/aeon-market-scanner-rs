use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CoinbaseOrderBookResponse {
    pub bids: Vec<serde_json::Value>, // [price, quantity, order_count]
    pub asks: Vec<serde_json::Value>, // [price, quantity, order_count]
}

#[derive(Debug, Deserialize)]
pub struct CoinbaseTickerWs {
    #[serde(rename = "type")]
    pub msg_type: String,
    #[serde(rename = "best_bid")]
    pub best_bid: String,
    #[serde(rename = "best_bid_size")]
    pub best_bid_size: String,
    #[serde(rename = "best_ask")]
    pub best_ask: String,
    #[serde(rename = "best_ask_size")]
    pub best_ask_size: String,
}
