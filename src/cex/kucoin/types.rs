use serde::Deserialize;

#[derive(Debug, Deserialize)]

pub struct KucoinOrderBookData {
    #[serde(rename = "bestBid")]
    pub best_bid: String,
    #[serde(rename = "bestBidSize")]
    pub best_bid_size: String,
    #[serde(rename = "bestAsk")]
    pub best_ask: String,
    #[serde(rename = "bestAskSize")]
    pub best_ask_size: String,
}
