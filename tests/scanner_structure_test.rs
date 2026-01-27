mod scanner_common;
use scanner_common::TEST_SYMBOL;
use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use aeon_market_scanner_rs::CexExchange;

#[tokio::test]
async fn test_arbitrage_opportunity_structure_bnbusdt() {
    println!("===== Testing ArbitrageOpportunity Structure for BNBUSDT =====\n");

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
    println!("  Buy from: {}", opp.buy_exchange);
    println!("  Sell on: {}", opp.sell_exchange);
    println!("  Profit: {:.4}%", opp.profit_percentage);
    println!("  Profit amount: ${:.4}", opp.profit);

    // Verify all fields are populated
    assert!(
        !opp.buy_exchange.is_empty(),
        "Buy exchange should not be empty"
    );
    assert!(
        !opp.sell_exchange.is_empty(),
        "Sell exchange should not be empty"
    );
    assert_eq!(opp.symbol, TEST_SYMBOL, "Symbol should match");
    assert_ne!(
        opp.buy_exchange, opp.sell_exchange,
        "Buy and sell exchanges should be different"
    );

    // Verify price data: opp.buy_price / opp.sell_price are effective (commission); raw in price_data
    match &opp.buy_price_data {
        PriceData::Cex(cex_price) => {
            assert_eq!(cex_price.symbol, TEST_SYMBOL);
            assert!(
                opp.buy_price >= cex_price.ask_price,
                "Effective buy (ask+commission) >= raw ask"
            );
            assert!(cex_price.ask_qty > 0.0, "Ask quantity should be positive");
            assert!(cex_price.timestamp > 0, "Timestamp should be present");
            assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            println!(
                "  Buy: raw ask={:.4}, effective={:.4}",
                cex_price.ask_price, opp.buy_price
            );
        }
        PriceData::Dex(_) => {}
    }

    match &opp.sell_price_data {
        PriceData::Cex(cex_price) => {
            assert_eq!(cex_price.symbol, TEST_SYMBOL);
            assert!(
                opp.sell_price <= cex_price.bid_price,
                "Effective sell (bid−commission) <= raw bid"
            );
            assert!(cex_price.bid_qty > 0.0, "Bid quantity should be positive");
            assert!(cex_price.timestamp > 0, "Timestamp should be present");
            assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            println!(
                "  Sell: raw bid={:.4}, effective={:.4}",
                cex_price.bid_price, opp.sell_price
            );
        }
        PriceData::Dex(_) => {}
    }

    // Test total_profit calculation
    let calculated_total = opp.total_profit();
    let expected_total = opp.profit * opp.buy_quantity.min(opp.sell_quantity);
    assert!(
        (calculated_total - expected_total).abs() < 0.0001,
        "Total profit calculation should be correct"
    );

    println!(
        "✓ ArbitrageOpportunity structure test passed for {}\n",
        TEST_SYMBOL
    );
}
