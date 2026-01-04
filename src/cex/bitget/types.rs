use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BitgetOrderBookResponse {
    #[serde(default)]
    pub data: Option<BitgetOrderBookData>,
}

#[derive(Debug, Deserialize)]
pub struct BitgetOrderBookData {
    #[serde(default)]
    pub asks: Vec<[String; 2]>, // [price, quantity]
    pub bids: Vec<[String; 2]>, // [price, quantity]
}
