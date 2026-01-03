use aeon_market_scanner_rs::{CexExchange, Exchange, ExchangeTrait, MarketScannerError, Mexc};

#[tokio::test]
async fn test_mexc_health_check() {
    let mexc = Mexc::new();
    let result = mexc.health_check().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_mexc_get_price() {
    let mexc = Mexc::new();
    println!("Testing Mexc get price for BTCUSDT");
    let result = mexc.get_price("BTCUSDT").await;
    assert!(result.is_ok());

    let price = result.unwrap();

    // Check symbol
    assert_eq!(price.symbol, "BTCUSDT");
    println!("   Symbol: {}", price.symbol);

    // Check bid price
    assert!(price.bid_price > 0.0, "Bid price should be positive");
    println!("   Bid Price: ${}", price.bid_price);

    // Check ask price
    assert!(price.ask_price > 0.0, "Ask price should be positive");
    println!("   Ask Price: ${}", price.ask_price);

    // Check mid price
    assert!(price.mid_price > 0.0, "Mid price should be positive");
    println!("   Mid Price: ${}", price.mid_price);

    // Check mid price is between bid and ask price
    assert!(price.mid_price >= price.bid_price);
    assert!(price.mid_price <= price.ask_price);

    // Check bid quantity
    assert!(price.bid_qty > 0.0, "Bid quantity should be positive");
    println!("   Bid Quantity: {}", price.bid_qty);

    // Check ask quantity
    assert!(price.ask_qty > 0.0, "Ask quantity should be positive");
    println!("   Ask Quantity: {}", price.ask_qty);

    // Check timestamp
    assert!(price.timestamp > 0, "Timestamp should be positive");
    println!("   Timestamp: {}", price.timestamp);

    // Check exchange
    assert_eq!(
        price.exchange,
        Exchange::Cex(CexExchange::MEXC),
        "Exchange should be Mexc"
    );
    println!("   Exchange: {:?}", price.exchange);
}
