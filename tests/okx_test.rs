// tests/okx_test.rs
mod common;

use aeon_market_scanner_rs::{CexExchange, Exchange, OKX};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_okx_health_check() {
    let okx = OKX::new();
    test_health_check_common(&okx, "OKX").await;
}

#[tokio::test]
async fn test_okx_get_price() {
    let okx = OKX::new();
    test_get_price_common(&okx, "BTCUSDT", Exchange::Cex(CexExchange::OKX), "OKX").await;
}

#[tokio::test]
async fn test_okx_get_price_invalid_symbol() {
    let okx = OKX::new();
    test_get_price_invalid_symbol_common(&okx, "OKX").await;
}

#[tokio::test]
async fn test_okx_get_price_empty_symbol() {
    let okx = OKX::new();
    test_get_price_empty_symbol_common(&okx, "OKX").await;
}
