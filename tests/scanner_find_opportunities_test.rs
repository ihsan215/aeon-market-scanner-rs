use aeon_market_scanner_rs::common::{AggregatorPrice, CexExchange, CexPrice, Exchange};
use aeon_market_scanner_rs::dex::{ChainId, DexPrice, PoolKind, PriceDirection};
use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};

#[test]
fn test_find_opportunities_mock_data() {
    println!("\n=== Testing find_opportunities with MOCK DATA (CEX, DEX, POOL) ===");

    let symbol = "BNBUSDT".to_string();

    // 1. Mock CEX Prices
    // We will create two CEXs. One has a high bid (good to sell to), one has a low ask (good to buy from).
    let cex_prices = vec![
        CexPrice {
            symbol: symbol.clone(),
            mid_price: 300.0,
            bid_price: 299.0,
            ask_price: 301.0, // Buying from Binance costs 301.0
            bid_qty: 10.0,
            ask_qty: 10.0,
            timestamp: 1600000000,
            exchange: Exchange::Cex(CexExchange::Binance),
        },
        CexPrice {
            symbol: symbol.clone(),
            mid_price: 310.0,
            bid_price: 309.0, // Selling to OKX yields 309.0
            ask_price: 311.0,
            bid_qty: 5.0,
            ask_qty: 5.0,
            timestamp: 1600000000,
            exchange: Exchange::Cex(CexExchange::OKX),
        },
    ];

    // 2. Mock DEX Aggregator Price
    // Aggregator offers an amazing buy price (very low ask)
    let dex_prices = vec![AggregatorPrice {
        symbol: "WBNBUSDT".to_string(),
        mid_price: 280.0,
        bid_price: 279.0,
        ask_price: 281.0, // Buying from KyberSwap costs 281.0 (Best buy!)
        bid_qty: 50.0,
        ask_qty: 50.0,
        timestamp: 1600000000,
        exchange: Exchange::Dex(aeon_market_scanner_rs::common::DexAggregator::KyberSwap),
        bid_route_summary: None,
        ask_route_summary: None,
        bid_route_data: None,
        ask_route_data: None,
    }];

    // 3. Mock Pool Listener Price
    // The pool is currently trading at a very high price (great to sell to)
    let pool_prices = vec![DexPrice {
        chain_id: ChainId::BSC,
        pool_address: "0xMockPool".to_string(),
        pool_kind: PoolKind::V3Uniswap,
        bid: 318.0,
        ask: 322.0,
        mid: 320.0, // Selling to Pool yields 320.0 (Best sell!)
        direction: PriceDirection::Token0PerToken1,
        sqrt_price_x96: None,
        block_number: 1234567,
        timestamp: 1600000000,
        symbol: Some(symbol.clone()),
        pool_id: None,
    }];

    // Run the scanner with all 3 sources simultaneously
    let mut opportunities = ArbitrageScanner::find_opportunities(
        &cex_prices,
        Some(&dex_prices),
        Some(&pool_prices),
        None, // No custom fee overrides, it will use defaults (0.1% for CEX/DEX, 0% for pool)
    );

    // Sort by net spread percentage
    opportunities.sort_by(|a, b| {
        b.net_spread_percentage
            .partial_cmp(&a.net_spread_percentage)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    assert!(
        !opportunities.is_empty(),
        "It should have found multiple arbitrage opportunities."
    );

    println!("Found {} opportunities.", opportunities.len());
    for (i, opp) in opportunities.iter().enumerate() {
        println!(
            "#{}: Buy on {} -> Sell on {} | Net Spread: ${:.2} ({:.2}%)",
            i + 1,
            opp.source_exchange,
            opp.destination_exchange,
            opp.net_spread,
            opp.net_spread_percentage
        );
    }

    let best = &opportunities[0];
    println!("\nBest Arbitrage Opportunity Full Object: {:#?}", best);

    // The best path should be: Buy KyberSwap (281) -> Sell Pool (pool bid = 318)
    assert_eq!(best.source_exchange, "KyberSwap");
    assert_eq!(best.destination_exchange, "Pool:BSC:0xMockPool");

    // Validate that the math checks out!
    // Buy KyberSwap Ask (281.0) with default 0.0% fee (since no override)
    // Sell Pool Bid (318.0) with default 0.0% fee
    // Net Spread = 318.0 - 281.0 = 37.0
    assert!(best.net_spread >= 37.0 && best.net_spread < 38.0);
    assert!(best.effective_bid > best.effective_ask);

    // Ensure it's correctly identifying the enum PriceData origins
    assert!(matches!(best.source_leg, PriceData::Dex(_)));
    assert!(matches!(best.destination_leg, PriceData::PoolListener(_)));

    // Let's check another path from the results: Buy KyberSwap -> Sell OKX
    let okx_path = opportunities
        .iter()
        .find(|o| o.destination_exchange == "OKX")
        .unwrap();
    assert_eq!(okx_path.source_exchange, "KyberSwap");
    // Buy Kyber (281.0) -> Sell OKX (309.0 * 0.999 = 308.691)
    // Net Spread = 308.691 - 281.0 = 27.691
    assert!(okx_path.net_spread > 27.0 && okx_path.net_spread < 28.0);
}
