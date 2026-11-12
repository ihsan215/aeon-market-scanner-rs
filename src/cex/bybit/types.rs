use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct BybitTickerData {
    pub symbol: String,
    #[serde(rename = "bid1Price")]
    pub bid1_price: String,
    #[serde(rename = "bid1Size")]
    pub bid1_size: String,
    #[serde(rename = "ask1Price")]
    pub ask1_price: String,
    #[serde(rename = "ask1Size")]
    pub ask1_size: String,
}

/// WebSocket orderbook snapshot (orderbook.1) for spot.
#[derive(Debug, Deserialize)]
pub struct BybitOrderbookSnapshot {
    #[serde(rename = "s")]
    #[allow(dead_code)]
    pub symbol: String,
    /// Bids: [[price, size], ...], descending by price.
    #[serde(rename = "b")]
    pub bids: Vec<[String; 2]>,
    /// Asks: [[price, size], ...], ascending by price.
    #[serde(rename = "a")]
    pub asks: Vec<[String; 2]>,
}

#[derive(Debug, Deserialize)]
pub struct BybitOrderbookWsMessage {
    #[allow(dead_code)]
    pub topic: String,
    #[serde(rename = "type")]
    pub msg_type: String,
    pub data: BybitOrderbookSnapshot,
    /// System time the message was generated (ms).
    #[serde(default)]
    pub ts: Option<u64>,
    /// Matching-engine time of this order-book state (ms).
    #[serde(default)]
    pub cts: Option<u64>,
}
