use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct HtxOrderBookResponse {
    pub tick: HtxOrderBookData,
}

#[derive(Debug, Deserialize)]
pub struct HtxOrderBookData {
    pub bids: Vec<[f64; 2]>, // [price, quantity] - HTX returns numbers, not strings
    pub asks: Vec<[f64; 2]>, // [price, quantity]
}
