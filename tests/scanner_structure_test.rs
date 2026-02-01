mod scanner_common;
use aeon_market_scanner_rs::CexExchange;
use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use scanner_common::TEST_SYMBOL;

#[tokio::test]
async fn test_arbitrage_opportunity_structure_ethusdt() {
    println!("===== Testing ArbitrageOpportunity Structure for ETHUSDT =====\n");

    let cex_exchanges = vec![
        CexExchange::Binance,
        CexExchange::Bybit,
        CexExchange::OKX,
        CexExchange::Gateio,
    ];

    let result = ArbitrageScanner::scan_arbitrage_opportunities(
        TEST_SYMBOL,
        &cex_exchanges,
        None,
        None,
        None,
        None,
    )
    .await;

    assert!(result.is_ok(), "Should successfully scan arbitrage");

    let opportunities = result.unwrap();

    if opportunities.is_empty() {
        println!("No arbitrage opportunities found (this is normal if prices are similar)");
        return;
    }

    // Test first opportunity structure (most profitable)
    let opp = &opportunities[0];

    println!("Most profitable opportunity for {}:", TEST_SYMBOL);
    println!("  Source (buy): {}", opp.source_exchange);
    println!("  Destination (sell): {}", opp.destination_exchange);
    println!("  Spread: {:.4}%", opp.spread_percentage);
    println!("  Spread amount: ${:.4}", opp.spread);
    println!(
        "  Source commission: {:.4}% | Dest: {:.4}% | Total commission (USD): ${:.4}",
        opp.source_commission_percent, opp.destination_commission_percent, opp.total_commission
    );

    // Verify all fields are populated
    assert!(
        !opp.source_exchange.is_empty(),
        "Source exchange should not be empty"
    );
    assert!(
        !opp.destination_exchange.is_empty(),
        "Destination exchange should not be empty"
    );
    assert_eq!(opp.symbol, TEST_SYMBOL, "Symbol should match");
    assert_ne!(
        opp.source_exchange, opp.destination_exchange,
        "Source and destination exchanges should be different"
    );

    // Verify price data: effective_ask/effective_bid include commission; raw in legs
    match &opp.source_leg {
        PriceData::Cex(cex_price) => {
            assert_eq!(cex_price.symbol, TEST_SYMBOL);
            assert!(
                opp.effective_ask >= cex_price.ask_price,
                "Effective ask (raw ask + commission) >= raw ask"
            );
            assert!(cex_price.ask_qty > 0.0, "Ask quantity should be positive");
            assert!(cex_price.timestamp > 0, "Timestamp should be present");
            assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            println!(
                "  Source: raw ask={:.4}, effective_ask={:.4}",
                cex_price.ask_price, opp.effective_ask
            );
        }
        PriceData::Dex(_) => {}
    }

    match &opp.destination_leg {
        PriceData::Cex(cex_price) => {
            assert_eq!(cex_price.symbol, TEST_SYMBOL);
            assert!(
                opp.effective_bid <= cex_price.bid_price,
                "Effective bid (raw bid − commission) <= raw bid"
            );
            assert!(cex_price.bid_qty > 0.0, "Bid quantity should be positive");
            assert!(cex_price.timestamp > 0, "Timestamp should be present");
            assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            println!(
                "  Destination: raw bid={:.4}, effective_bid={:.4}",
                cex_price.bid_price, opp.effective_bid
            );
        }
        PriceData::Dex(_) => {}
    }

    // Test total_profit calculation
    let calculated_total = opp.total_profit();
    let expected_total = opp.spread * opp.executable_quantity;
    assert!(
        (calculated_total - expected_total).abs() < 0.0001,
        "Total profit calculation should be correct"
    );

    println!(
        "✓ ArbitrageOpportunity structure test passed for {}\n",
        TEST_SYMBOL
    );
}
