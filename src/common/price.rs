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
    pub timestamp: u64,
    pub exchange: Exchange,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPrice {
    pub symbol: String,
    pub mid_price: f64,
    pub bid_price: f64,
    pub ask_price: f64,
    pub bid_qty: f64,
    pub ask_qty: f64,
    pub timestamp: u64,
    pub exchange: Exchange,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_route_summary: Option<DexRouteSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_route_summary: Option<DexRouteSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bid_route_data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask_route_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexRouteSummary {
    pub token_in: String,
    pub token_out: String,
    pub amount_in: String,
    pub amount_out: String,
}
