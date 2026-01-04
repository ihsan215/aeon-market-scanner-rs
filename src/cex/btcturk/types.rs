use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BtcturkOrderBookResponse {
    pub data: BtcturkOrderBookData,
}

#[derive(Debug, Deserialize)]
pub struct BtcturkOrderBookData {
    #[serde(default)]
    pub bids: Vec<[String; 2]>, // [price, quantity]
    pub asks: Vec<[String; 2]>, // [price, quantity]
}
