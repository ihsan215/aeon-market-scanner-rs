mod common;

use aeon_market_scanner_rs::{Btcturk, CexExchange, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_btcturk_health_check() {
    test_health_check_common(&Btcturk::new(), "BTCTurk").await;
}

#[tokio::test]
async fn test_btcturk_get_price() {
    test_get_price_common(
        &Btcturk::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Btcturk),
        "BTCTurk",
    )
    .await;
}

#[tokio::test]
async fn test_btcturk_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Btcturk::new(), "BTCTurk").await;
}

#[tokio::test]
async fn test_btcturk_empty_symbol() {
    test_get_price_empty_symbol_common(&Btcturk::new(), "BTCTurk").await;
}
