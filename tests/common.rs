use aeon_market_scanner_rs::{CEXTrait, Exchange, MarketScannerError};

pub async fn test_health_check_common<T: CEXTrait>(exchange: &T, exchange_name: &str) {
    let result = exchange.health_check().await;
    assert!(
        result.is_ok(),
        "{} health check should succeed",
        exchange_name
    );
    println!("{} health check passed", exchange_name);
}

pub async fn test_get_price_common<T: CEXTrait>(
    exchange: &T,
    symbol: &str,
    expected_exchange: Exchange,
    exchange_name: &str,
) {
    let result = exchange.get_price(symbol).await;
    assert!(result.is_ok(), "Should be able to get {} price", symbol);

    let price = result.unwrap();

    // Check symbol
    assert_eq!(price.symbol, symbol);
    println!("Symbol: {}", price.symbol);

    // Check bid price
    assert!(price.bid_price > 0.0, "Bid price should be positive");
    println!("Bid Price: ${}", price.bid_price);

    // Check ask price
    assert!(price.ask_price > 0.0, "Ask price should be positive");
    println!("Ask Price: ${}", price.ask_price);

    // Check mid price
    assert!(price.mid_price > 0.0, "Mid price should be positive");
    println!("Mid Price: ${}", price.mid_price);

    // Check mid price is between bid and ask price
    assert!(price.mid_price >= price.bid_price);
    assert!(price.mid_price <= price.ask_price);

    // Check bid quantity
    assert!(price.bid_qty > 0.0, "Bid quantity should be positive");
    println!("Bid Quantity: {}", price.bid_qty);

    // Check ask quantity
    assert!(price.ask_qty > 0.0, "Ask quantity should be positive");
    println!("Ask Quantity: {}", price.ask_qty);

    // Check timestamp
    assert!(price.timestamp > 0, "Timestamp should be positive");
    println!("Timestamp: {}", price.timestamp);

    // Check exchange
    assert_eq!(
        price.exchange, expected_exchange,
        "Exchange should be {}",
        exchange_name
    );
    println!("Exchange: {:?}", price.exchange);
}

pub async fn test_get_price_invalid_symbol_common<T: CEXTrait>(exchange: &T, exchange_name: &str) {
    let invalid_symbols = vec!["INVALID123", "XYZABC", "NOTREAL", "FAKESYMBOL"];

    for symbol in invalid_symbols {
        println!("Testing symbol: {}", symbol);
        let result = exchange.get_price(symbol).await;

        assert!(
            result.is_err(),
            "Invalid symbol '{}' should return error for {}",
            symbol,
            exchange_name
        );

        match result {
            Err(MarketScannerError::ApiError(msg)) => {
                println!("Got API error: {}", msg);
                assert!(
                    msg.contains(&format!("{} API error", exchange_name)),
                    "Error message should contain '{} API error'",
                    exchange_name
                );
            }
            Err(e) => {
                println!("Unexpected error type: {:?}", e);
                panic!(
                    "Expected ApiError for invalid symbol '{}' in {}, got: {:?}",
                    symbol, exchange_name, e
                );
            }
            Ok(price) => {
                println!("Unexpected success: {:?}", price);
                panic!(
                    "Expected error for invalid symbol '{}' in {}, got price: {:?}",
                    symbol, exchange_name, price
                );
            }
        }
    }
    println!("{} invalid symbol test passed\n", exchange_name);
}

pub async fn test_get_price_empty_symbol_common<T: CEXTrait>(exchange: &T, exchange_name: &str) {
    let result = exchange.get_price("").await;

    assert!(
        result.is_err(),
        "Empty symbol should return error for {}",
        exchange_name
    );

    match result {
        Err(MarketScannerError::ApiError(msg)) => {
            println!("Got API error: {}", msg);
            assert!(
                msg.contains(&format!("{} API error", exchange_name)),
                "Error message should contain '{} API error'",
                exchange_name
            );
        }
        Err(e) => {
            println!("Got error: {:?}", e);
            // For empty symbols, it may be ApiError or another error type
        }
        Ok(price) => {
            println!("Unexpected success: {:?}", price);
            panic!(
                "Expected error for empty symbol in {}, got price: {:?}",
                exchange_name, price
            );
        }
    }
    println!("{} empty symbol test passed\n", exchange_name);
}
