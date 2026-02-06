use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BinanceBookTickerResponse {
    pub symbol: String,
    #[serde(rename = "bidPrice")]
    pub bid_price: String,
    #[serde(rename = "bidQty")]
    pub bid_qty: String,
    #[serde(rename = "askPrice")]
    pub ask_price: String,
    #[serde(rename = "askQty")]
    pub ask_qty: String,
}

/// WebSocket bookTicker stream payload (Binance uses single-letter keys).
/// Stream: wss://stream.binance.com:9443/ws/<symbol>@bookTicker
#[derive(Debug, Deserialize)]
#[allow(non_snake_case)]
pub struct BinanceBookTickerWs {
    pub s: String, // symbol
    pub b: String, // best bid price
    pub B: String, // best bid qty
    pub a: String, // best ask price
    pub A: String, // best ask qty
}
