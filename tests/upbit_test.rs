mod common;
use aeon_market_scanner_rs::{Upbit, CexExchange, Exchange, ExchangeTrait};
use common::{
    test_get_price_common, test_get_price_empty_symbol_common,
    test_get_price_invalid_symbol_common, test_health_check_common,
};

#[tokio::test]
async fn test_upbit_health_check() {
    test_health_check_common(&Upbit::new(), "Upbit").await;
}

#[tokio::test]
async fn test_upbit_get_price() {
    // Upbit uses KRW-BTC format, so BTCUSD gets converted to KRW-BTC
    test_get_price_common(
        &Upbit::new(),
        "BTCUSD",
        Exchange::Cex(CexExchange::Upbit),
        "Upbit",
    )
    .await;
}

#[tokio::test]
async fn test_upbit_get_price_usdt() {
    // BTCUSDT -> USDT-BTC conversion
    let exchange = Upbit::new();
    let result = exchange.get_price("BTCUSDT").await;
    
    assert!(result.is_ok(), "Should be able to get BTCUSDT price (converted to USDT-BTC)");
    let price = result.unwrap();
    
    // Verify price data is valid
    assert!(price.bid_price > 0.0);
    assert!(price.ask_price > 0.0);
    assert!(price.mid_price > 0.0);
    assert!(price.mid_price >= price.bid_price);
    assert!(price.mid_price <= price.ask_price);
    
    println!("BTCUSDT converted to USDT-BTC: OK");
    println!("Symbol: {}, Bid: ${}, Ask: ${}, Mid: ${}", 
             price.symbol, price.bid_price, price.ask_price, price.mid_price);
}

#[tokio::test]
async fn test_upbit_invalid_symbol() {
    test_get_price_invalid_symbol_common(&Upbit::new(), "Upbit").await;
}

#[tokio::test]
async fn test_upbit_empty_symbol() {
    test_get_price_empty_symbol_common(&Upbit::new(), "Upbit").await;
}

