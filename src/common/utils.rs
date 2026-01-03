// src/common/utils.rs
use crate::common::MarketScannerError;

// Parse a string to a f64, return a MarketScannerError if the parsing fails
pub fn parse_f64(value: &str, field_name: &str) -> Result<f64, MarketScannerError> {
    value
        .parse::<f64>()
        .map_err(|_| MarketScannerError::ApiError(format!("Invalid {} format", field_name)))
}

// Find mid price between bid and ask price
pub fn find_mid_price(bid_price: f64, ask_price: f64) -> f64 {
    (bid_price + ask_price) / 2.0
}

// get timestamp in milliseconds
pub fn get_timestamp_millis() -> u64 {
    chrono::Utc::now()
        .timestamp_millis()
        .try_into()
        .unwrap_or(0)
}
