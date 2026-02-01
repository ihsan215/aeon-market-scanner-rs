mod scanner_common;

use aeon_market_scanner_rs::{
    DEXTrait, DexAggregator, DexRouteSummary, Exchange, ExchangeTrait, KyberSwap,
};
use scanner_common::{
    create_base_eth, create_base_usdc, create_bsc_bnb, create_bsc_usdt, create_eth_eth,
    create_eth_usdt,
};
use std::time::Duration;

const QUOTE_AMOUNT: f64 = 1000.0;
/// Delay before API-calling tests to avoid KyberSwap rate limiting.
/// For best effect run with: cargo test kyberswap -- --test-threads=1
const DELAY_BETWEEN_TESTS: Duration = Duration::from_secs(2);

fn print_route_summary(label: &str, route: &DexRouteSummary) {
    println!("\n=== {} Route Summary ===", label);
    println!(
        "Token In: {}, Token Out: {}",
        route.token_in, route.token_out
    );
    println!(
        "Amount In: {}, Amount Out: {}",
        route.amount_in, route.amount_out
    );
    println!(
        "Amount In Wei: {}, Amount Out Wei: {}",
        route.amount_in_wei, route.amount_out_wei
    );
    if let Some(ref gas) = route.gas {
        println!("Gas: {}", gas);
    }
    if let Some(ref gas_price) = route.gas_price {
        println!("Gas Price (wei): {}", gas_price);
    }
    if let Some(gas_usd) = route.gas_usd {
        println!("Gas USD: ${}", gas_usd);
    }
}

#[tokio::test]
async fn test_kyberswap_health_check() {
    tokio::time::sleep(DELAY_BETWEEN_TESTS).await;
    let exchange = KyberSwap::new();
    let result = exchange.health_check().await;

    assert!(
        result.is_ok(),
        "KyberSwap health check should succeed: {:?}",
        result.err()
    );

    println!("KyberSwap health check passed");
}

#[tokio::test]
async fn test_kyberswap_exchange_name() {
    let exchange = KyberSwap::new();
    assert_eq!(exchange.exchange_name(), "KyberSwap");
    println!(
        "KyberSwap exchange name verified: {}",
        exchange.exchange_name()
    );
}

#[tokio::test]
async fn test_kyberswap_get_price_ethereum() {
    tokio::time::sleep(DELAY_BETWEEN_TESTS).await;
    let exchange = KyberSwap::new();
    let eth_token = create_eth_eth();
    let usdt_token = create_eth_usdt();
    let result = exchange
        .get_price(&eth_token, &usdt_token, QUOTE_AMOUNT)
        .await;

    if let Err(e) = &result {
        println!("Error getting price: {:?}", e);
    }
    assert!(
        result.is_ok(),
        "Should be able to get ETHUSDT price on Ethereum: {:?}",
        result.err()
    );
    let price = result.unwrap();

    // Check symbol
    assert_eq!(price.symbol, "ETHUSDT");
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
    // Note: In DEX, bid (buying) is typically higher than ask (selling) due to slippage/spread
    // So mid should be: min(bid, ask) <= mid <= max(bid, ask)
    let min_price = price.bid_price.min(price.ask_price);
    let max_price = price.bid_price.max(price.ask_price);
    assert!(price.mid_price >= min_price && price.mid_price <= max_price);

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
    assert_eq!(price.exchange, Exchange::Dex(DexAggregator::KyberSwap));
    println!("Exchange: {:?}", price.exchange);

    // Log route summaries
    if let Some(ref bid_route) = price.bid_route_summary {
        print_route_summary("BID", bid_route);
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        print_route_summary("ASK", ask_route);
    }

    // Log full route data if available
    if let Some(ref bid_data) = price.bid_route_data {
        println!("\n=== BID Route Data (Full JSON) ===");
        println!(
            "{}",
            serde_json::to_string_pretty(bid_data)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );
    }
    if let Some(ref ask_data) = price.ask_route_data {
        println!("\n=== ASK Route Data (Full JSON) ===");
        println!(
            "{}",
            serde_json::to_string_pretty(ask_data)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );
    }
}

#[tokio::test]
async fn test_kyberswap_get_price_base() {
    tokio::time::sleep(DELAY_BETWEEN_TESTS).await;
    let exchange = KyberSwap::new();
    let eth_token = create_base_eth();
    let usdc_token = create_base_usdc();
    let result = exchange
        .get_price(&eth_token, &usdc_token, QUOTE_AMOUNT)
        .await;

    assert!(
        result.is_ok(),
        "Should be able to get ETHUSDC price on Base: {:?}",
        result.err()
    );
    let price = result.unwrap();

    // Check symbol
    assert_eq!(price.symbol, "ETHUSDC");
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

    // Check exchange
    assert_eq!(price.exchange, Exchange::Dex(DexAggregator::KyberSwap));
    println!("Exchange: {:?}", price.exchange);

    // Log route summaries
    if let Some(ref bid_route) = price.bid_route_summary {
        print_route_summary("BID", bid_route);
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        print_route_summary("ASK", ask_route);
    }
}

#[tokio::test]
async fn test_kyberswap_get_price_bsc() {
    tokio::time::sleep(DELAY_BETWEEN_TESTS).await;
    println!("===== BSC CHAIN TEST =======");

    let exchange = KyberSwap::new();
    let bnb_token = create_bsc_bnb();
    let usdt_token = create_bsc_usdt();
    let result = exchange
        .get_price(&bnb_token, &usdt_token, QUOTE_AMOUNT)
        .await;

    assert!(result.is_ok(), "Should be able to get BNBUSDT price on BSC");
    let price = result.unwrap();

    // Check symbol
    assert_eq!(price.symbol, "BNBUSDT");
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

    // Check exchange
    assert_eq!(price.exchange, Exchange::Dex(DexAggregator::KyberSwap));
    println!("Exchange: {:?}", price.exchange);

    // Log route summaries
    if let Some(ref bid_route) = price.bid_route_summary {
        print_route_summary("BID", bid_route);
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        print_route_summary("ASK", ask_route);
    }

    // Log full route data if available
    if let Some(ref bid_data) = price.bid_route_data {
        println!("\n=== BID Route Data (Full JSON) ===");
        println!(
            "{}",
            serde_json::to_string_pretty(bid_data)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );
    }
    if let Some(ref ask_data) = price.ask_route_data {
        println!("\n=== ASK Route Data (Full JSON) ===");
        println!(
            "{}",
            serde_json::to_string_pretty(ask_data)
                .unwrap_or_else(|_| "Failed to serialize".to_string())
        );
    }
}

#[tokio::test]
async fn test_kyberswap_get_price_different_chains() {
    // No delay: fails at validation before API call
    let exchange = KyberSwap::new();
    let eth_token = create_eth_eth();
    let usdc_token = create_base_usdc();
    let result = exchange
        .get_price(&eth_token, &usdc_token, QUOTE_AMOUNT)
        .await;

    assert!(
        result.is_err(),
        "Tokens on different chains should return error"
    );

    if let Err(e) = result {
        println!("Got expected error: {:?}", e);
    }
}
