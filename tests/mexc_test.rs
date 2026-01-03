mod common;
use aeon_market_scanner_rs::{CexExchange, Exchange, Mexc};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_mexc_health_check() {
    test_health_check_common(&Mexc::new(), "Mexc").await;
}

#[tokio::test]
async fn test_mexc_get_price() {
    test_get_price_common(
        &Mexc::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::MEXC),
        "Mexc",
    )
    .await;
}

#[tokio::test]
async fn test_mexc_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Mexc::new(), "Mexc").await;
}

#[tokio::test]
async fn test_mexc_empty_symbol() {
    test_get_price_empty_symbol_common(&Mexc::new(), "Mexc").await;
}
