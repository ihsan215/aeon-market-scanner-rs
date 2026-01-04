use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CoinbaseOrderBookResponse {
    pub bids: Vec<serde_json::Value>, // [price, quantity, order_count]
    pub asks: Vec<serde_json::Value>, // [price, quantity, order_count]
}
