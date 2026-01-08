mod common;

use aeon_market_scanner_rs::{CexExchange, Cryptocom, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_cryptocom_health_check() {
    test_health_check_common(&Cryptocom::new(), "Crypto.com").await;
}

#[tokio::test]
async fn test_cryptocom_get_price() {
    test_get_price_common(
        &Cryptocom::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Cryptocom),
        "Crypto.com",
    )
    .await;
}

#[tokio::test]
async fn test_cryptocom_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Cryptocom::new(), "Crypto.com").await;
}

#[tokio::test]
async fn test_cryptocom_empty_symbol() {
    test_get_price_empty_symbol_common(&Cryptocom::new(), "Crypto.com").await;
}
