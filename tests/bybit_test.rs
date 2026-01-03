mod common;

use aeon_market_scanner_rs::{Bybit, CexExchange, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_bybit_health_check() {
    let bybit = Bybit::new();
    test_health_check_common(&bybit, "Bybit").await;
}

#[tokio::test]
async fn test_bybit_get_price() {
    let bybit = Bybit::new();
    test_get_price_common(
        &bybit,
        "BTCUSDT",
        Exchange::Cex(CexExchange::Bybit),
        "Bybit",
    )
    .await;
}

#[tokio::test]
async fn test_bybit_get_price_invalid_symbol() {
    let bybit = Bybit::new();
    test_get_price_invalid_symbol_common(&bybit, "Bybit").await;
}

#[tokio::test]
async fn test_bybit_get_price_empty_symbol() {
    let bybit = Bybit::new();
    test_get_price_empty_symbol_common(&bybit, "Bybit").await;
}
