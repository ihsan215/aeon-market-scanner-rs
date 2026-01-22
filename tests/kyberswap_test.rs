use aeon_market_scanner_rs::dex::chains::{BaseTokens, BscTokens, EthereumTokens, TokenMap};
use aeon_market_scanner_rs::{DEXTrait, DexAggregator, Exchange, ExchangeTrait, KyberSwap};

const QUOTE_AMOUNT: f64 = 1000.0;

#[tokio::test]
async fn test_kyberswap_health_check() {
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
    let exchange = KyberSwap::new();
    let tokens = EthereumTokens::new();
    let eth_token = tokens.get(&TokenMap::ETH).unwrap().clone();
    let usdt_token = tokens.get(&TokenMap::USDT).unwrap().clone();
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
        println!("\n=== BID Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            bid_route.token_in, bid_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            bid_route.amount_in, bid_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            bid_route.amount_in_wei, bid_route.amount_out_wei
        );
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        println!("\n=== ASK Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            ask_route.token_in, ask_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            ask_route.amount_in, ask_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            ask_route.amount_in_wei, ask_route.amount_out_wei
        );
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
    let exchange = KyberSwap::new();
    let tokens = BaseTokens::new();
    let eth_token = tokens.get(&TokenMap::ETH).unwrap().clone();
    let usdc_token = tokens.get(&TokenMap::USDC).unwrap().clone();
    let result = exchange
        .get_price(&eth_token, &usdc_token, QUOTE_AMOUNT)
        .await;

    assert!(
        result.is_ok(),
        "Should be able to get ETHUSDC price on Base"
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
        println!("\n=== BID Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            bid_route.token_in, bid_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            bid_route.amount_in, bid_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            bid_route.amount_in_wei, bid_route.amount_out_wei
        );
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        println!("\n=== ASK Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            ask_route.token_in, ask_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            ask_route.amount_in, ask_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            ask_route.amount_in_wei, ask_route.amount_out_wei
        );
    }
}

#[tokio::test]
async fn test_kyberswap_get_price_bsc() {
    println!("===== BSC CHAIN TEST =======");

    let exchange = KyberSwap::new();
    let tokens = BscTokens::new();
    let bnb_token = tokens.get(&TokenMap::BNB).unwrap().clone();
    let usdt_token = tokens.get(&TokenMap::USDT).unwrap().clone();
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
        println!("\n=== BID Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            bid_route.token_in, bid_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            bid_route.amount_in, bid_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            bid_route.amount_in_wei, bid_route.amount_out_wei
        );
    }
    if let Some(ref ask_route) = price.ask_route_summary {
        println!("\n=== ASK Route Summary ===");
        println!(
            "Token In: {}, Token Out: {}",
            ask_route.token_in, ask_route.token_out
        );
        println!(
            "Amount In: {}, Amount Out: {}",
            ask_route.amount_in, ask_route.amount_out
        );
        println!(
            "Amount In Wei: {}, Amount Out Wei: {}",
            ask_route.amount_in_wei, ask_route.amount_out_wei
        );
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
    let exchange = KyberSwap::new();
    let eth_tokens = EthereumTokens::new();
    let base_tokens = BaseTokens::new();
    let eth_token = eth_tokens.get(&TokenMap::ETH).unwrap().clone();
    let usdc_token = base_tokens.get(&TokenMap::USDC).unwrap().clone();
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
