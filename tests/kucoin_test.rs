mod common;

use aeon_market_scanner_rs::{CexExchange, Exchange, Kucoin};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_kucoin_health_check() {
    test_health_check_common(&Kucoin::new(), "KuCoin").await;
}

#[tokio::test]
async fn test_kucoin_get_price() {
    test_get_price_common(
        &Kucoin::new(),
        "BTCUSDT",
        Exchange::Cex(CexExchange::Kucoin),
        "KuCoin",
    )
    .await;
}

#[tokio::test]
async fn test_kucoin_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Kucoin::new(), "KuCoin").await;
}

#[tokio::test]
async fn test_kucoin_empty_symbol() {
    test_get_price_empty_symbol_common(&Kucoin::new(), "KuCoin").await;
}
