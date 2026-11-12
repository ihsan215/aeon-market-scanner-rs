use crate::common::exchange::Exchange;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CexPrice {
    pub symbol: String,
    pub mid_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_qty: f64,
    pub ask_qty: f64,
    /// Local receive time (ms since epoch): when this library parsed the message.
    pub timestamp: u64,
    pub exchange: Exchange,
    /// Event time reported by the exchange itself (ms since epoch), when the venue provides one.
    /// Compare with `timestamp` to measure exchange → client latency or to spot lagging feeds.
    /// `None` for feeds without an event time (Binance bookTicker, Bitfinex ticker,
    /// Gate v3 depth, BtcTurk, HTX) and for REST prices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_timestamp: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorPrice {
    pub symbol: String,
    pub mid_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_qty: f64,
    pub ask_qty: f64,
    pub timestamp: u64,
    pub exchange: Exchange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_route_summary: Option<AggregatorRouteSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_route_summary: Option<AggregatorRouteSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_route_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_route_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatorRouteSummary {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: f64,
    pub amount_out: f64,
    pub amount_in_wei: String,
    pub amount_out_wei: String,
    /// Gas limit for the swap
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<String>,
    /// Gas price in wei
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<String>,
    /// Gas cost in USD
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_usd: Option<f64>,
}
