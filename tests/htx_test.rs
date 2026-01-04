mod common;

use aeon_market_scanner_rs::{CexExchange, Exchange, Htx};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_htx_health_check() {
    test_health_check_common(&Htx::new(), "HTX").await;
}

#[tokio::test]
async fn test_htx_get_price() {
    test_get_price_common(
        &Htx::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Htx),
        "HTX",
    )
    .await;
}

#[tokio::test]
async fn test_htx_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Htx::new(), "HTX").await;
}

#[tokio::test]
async fn test_htx_empty_symbol() {
    test_get_price_empty_symbol_common(&Htx::new(), "HTX").await;
}
