mod common;

use aeon_market_scanner_rs::{CexExchange, Exchange, Gateio};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_gateio_health_check() {
    test_health_check_common(&Gateio::new(), "Gate.io").await;
}

#[tokio::test]
async fn test_gateio_get_price() {
    test_get_price_common(
        &Gateio::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Gateio),
        "Gate.io",
    )
    .await;
}

#[tokio::test]
async fn test_gateio_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Gateio::new(), "Gate.io").await;
}

#[tokio::test]
async fn test_gateio_empty_symbol() {
    test_get_price_empty_symbol_common(&Gateio::new(), "Gate.io").await;
}
