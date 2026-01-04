mod common;

use aeon_market_scanner_rs::{CexExchange, Coinbase, Exchange, ExchangeTrait};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_coinbase_health_check() {
    test_health_check_common(&Coinbase::new(), "Coinbase").await;
}

#[tokio::test]
async fn test_coinbase_get_price() {
    let exchange = Coinbase::new();
    let result = exchange.get_price("BTCUSDT").await;
    if let Err(e) = &result {
        eprintln!("Error getting BTCUSDT price: {:?}", e);
    }
    assert!(result.is_ok(), "Should be able to get BTCUSDT price");
    test_get_price_common(
        &exchange,
        "BTCUSDT",
        Exchange::Cex(CexExchange::Coinbase),
        "Coinbase",
    )
    .await;
}

#[tokio::test]
async fn test_coinbase_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Coinbase::new(), "Coinbase").await;
}

#[tokio::test]
async fn test_coinbase_empty_symbol() {
    test_get_price_empty_symbol_common(&Coinbase::new(), "Coinbase").await;
}
