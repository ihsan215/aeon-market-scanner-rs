mod common;
use aeon_market_scanner_rs::{Binance, CexExchange, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_binance_health_check() {
    test_health_check_common(&Binance::new(), "Binance").await;
}

#[tokio::test]
async fn test_binance_get_price() {
    test_get_price_common(
        &Binance::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Binance),
        "Binance",
    )
    .await;
}

#[tokio::test]
async fn test_binance_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Binance::new(), "Binance").await;
}

#[tokio::test]
async fn test_binance_empty_symbol() {
    test_get_price_empty_symbol_common(&Binance::new(), "Binance").await;
}
