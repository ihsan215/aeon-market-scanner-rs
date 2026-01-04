use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GateioOrderBookResponse {
    #[serde(default)]
    pub asks: Vec<[String; 2]>, // [price, quantity]
    pub bids: Vec<[String; 2]>, // [price, quantity]
}
