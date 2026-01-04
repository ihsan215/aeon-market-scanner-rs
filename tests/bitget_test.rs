mod common;

use aeon_market_scanner_rs::{Bitget, CexExchange, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_bitget_health_check() {
    test_health_check_common(&Bitget::new(), "Bitget").await;
}

#[tokio::test]
async fn test_bitget_get_price() {
    test_get_price_common(
        &Bitget::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Bitget),
        "Bitget",
    )
    .await;
}

#[tokio::test]
async fn test_bitget_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Bitget::new(), "Bitget").await;
}

#[tokio::test]
async fn test_bitget_empty_symbol() {
    test_get_price_empty_symbol_common(&Bitget::new(), "Bitget").await;
}
