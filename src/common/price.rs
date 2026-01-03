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
