use serde::Deserialize;

// Crypto.com Exchange API response types
#[derive(Debug, Deserialize)]
pub struct CryptocomOrderBookResponse {
    #[serde(rename = "result")]
    pub result: CryptocomOrderBookResult,
}

#[derive(Debug, Deserialize)]
pub struct CryptocomOrderBookResult {
    #[serde(rename = "data")]
    pub data: Vec<CryptocomOrderBookData>,
}

#[derive(Debug, Deserialize)]
pub struct CryptocomOrderBookData {
    #[serde(rename = "bids")]
    pub bids: Vec<[String; 3]>, // [price, quantity, count]
    #[serde(rename = "asks")]
    pub asks: Vec<[String; 3]>, // [price, quantity, count]
}
