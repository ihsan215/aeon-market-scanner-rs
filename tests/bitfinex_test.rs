mod common;
use aeon_market_scanner_rs::{Bitfinex, CEXTrait, CexExchange, Exchange};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_bitfinex_health_check() {
    test_health_check_common(&Bitfinex::new(), "Bitfinex").await;
}

#[tokio::test]
async fn test_bitfinex_get_price() {
    test_get_price_common(
        &Bitfinex::new(),
        "BTCUSD",
        Exchange::Cex(CexExchange::Bitfinex),
        "Bitfinex",
    )
    .await;
}

#[tokio::test]
async fn test_bitfinex_get_price_usdt() {
    // Bitfinex calls USDT "UST" (tBTCUST); the request is converted but the
    // returned symbol is the standard BTCUSDT so it matches other exchanges.
    let exchange = Bitfinex::new();
    let result = exchange.get_price("BTCUSDT").await;

    assert!(
        result.is_ok(),
        "Should be able to get BTCUSDT price (requested as tBTCUST)"
    );
    let price = result.unwrap();

    assert_eq!(
        price.symbol, "BTCUSDT",
        "Symbol should be the standard BTCUSDT, not Bitfinex's BTCUST"
    );

    // Verify price data is valid
    assert!(price.bid_price > 0.0);
    assert!(price.ask_price > 0.0);
    assert!(price.mid_price > 0.0);
    assert!(price.mid_price >= price.bid_price);
    assert!(price.mid_price <= price.ask_price);

    println!("BTCUSDT via tBTCUST: OK");
    println!(
        "Symbol: {}, Bid: ${}, Ask: ${}, Mid: ${}",
        price.symbol, price.bid_price, price.ask_price, price.mid_price
    );
}

#[tokio::test]
async fn test_bitfinex_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Bitfinex::new(), "Bitfinex").await;
}

#[tokio::test]
async fn test_bitfinex_empty_symbol() {
    test_get_price_empty_symbol_common(&Bitfinex::new(), "Bitfinex").await;
}
