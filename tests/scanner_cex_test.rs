mod scanner_common;
use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use scanner_common::{TEST_SYMBOL, get_all_cex_exchanges};

#[tokio::test]
async fn test_scan_cex_arbitrage_bnbusdt() {
    println!("===== Testing CEX Arbitrage Scanner for BNBUSDT =====\n");

    let cex_exchanges = get_all_cex_exchanges();
    println!(
        "Scanning {} CEX exchanges for {} arbitrage opportunities...\n",
        cex_exchanges.len(),
        TEST_SYMBOL
    );

    let result = ArbitrageScanner::scan_arbitrage_opportunities(
        TEST_SYMBOL,
        &cex_exchanges,
        None,
        None,
        None,
        None,
    )
    .await;

    assert!(
        result.is_ok(),
        "Should successfully scan CEX arbitrage opportunities: {:?}",
        result.err()
    );

    let opportunities = result.unwrap();
    println!(
        "Found {} arbitrage opportunities for {}\n",
        opportunities.len(),
        TEST_SYMBOL
    );

    if opportunities.is_empty() {
        println!("No arbitrage opportunities found (prices may be similar across exchanges)");
        return;
    }

    // Verify opportunities are sorted by profitability (descending - most profitable first)
    for (i, opp) in opportunities.iter().enumerate() {
        println!("Opportunity #{}:", i + 1);
        println!("  Buy from: {}", opp.buy_exchange);
        println!("  Sell on: {}", opp.sell_exchange);
        println!("  Symbol: {}", opp.symbol);
        println!("  Buy price: ${:.4}", opp.buy_price);
        println!("  Sell price: ${:.4}", opp.sell_price);
        println!("  Profit: ${:.4}", opp.profit);
        println!("  Profit %: {:.4}%", opp.profit_percentage);
        println!("  Buy quantity: {:.4}", opp.buy_quantity);
        println!("  Sell quantity: {:.4}", opp.sell_quantity);
        println!("  Total profit: ${:.4}", opp.total_profit());

        // Show full price data from buy and sell responses
        println!("  Buy Price Data:");
        match &opp.buy_price_data {
            PriceData::Cex(cex_price) => {
                println!("    Exchange: {:?}", cex_price.exchange);
                println!("    Symbol: {}", cex_price.symbol);
                println!("    Timestamp: {}", cex_price.timestamp);
                println!("    Bid Price: ${:.4}", cex_price.bid_price);
                println!("    Ask Price: ${:.4}", cex_price.ask_price);
                println!("    Mid Price: ${:.4}", cex_price.mid_price);
                println!("    Bid Quantity: {:.4}", cex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", cex_price.ask_qty);
            }
            PriceData::Dex(_) => {}
        }
        println!("  Sell Price Data:");
        match &opp.sell_price_data {
            PriceData::Cex(cex_price) => {
                println!("    Exchange: {:?}", cex_price.exchange);
                println!("    Symbol: {}", cex_price.symbol);
                println!("    Timestamp: {}", cex_price.timestamp);
                println!("    Bid Price: ${:.4}", cex_price.bid_price);
                println!("    Ask Price: ${:.4}", cex_price.ask_price);
                println!("    Mid Price: ${:.4}", cex_price.mid_price);
                println!("    Bid Quantity: {:.4}", cex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", cex_price.ask_qty);
            }
            PriceData::Dex(_) => {}
        }
        println!();

        // Verify profit is positive
        assert!(opp.profit > 0.0, "Profit should be positive");
        assert!(
            opp.profit_percentage > 0.0,
            "Profit percentage should be positive"
        );
        assert!(opp.buy_price > 0.0, "Buy price should be positive");
        assert!(opp.sell_price > 0.0, "Sell price should be positive");
        assert!(
            opp.sell_price > opp.buy_price,
            "Sell price should be higher than buy price"
        );

        // Verify price data is present and contains full response
        match &opp.buy_price_data {
            PriceData::Cex(cex_price) => {
                assert_eq!(cex_price.symbol, TEST_SYMBOL);
                assert_eq!(cex_price.ask_price, opp.buy_price);
                assert!(cex_price.timestamp > 0, "Timestamp should be present");
                assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            }
            PriceData::Dex(_) => {
                panic!("Buy price data should be CEX for CEX-only scan");
            }
        }

        match &opp.sell_price_data {
            PriceData::Cex(cex_price) => {
                assert_eq!(cex_price.symbol, TEST_SYMBOL);
                assert_eq!(cex_price.bid_price, opp.sell_price);
                assert!(cex_price.timestamp > 0, "Timestamp should be present");
                assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            }
            PriceData::Dex(_) => {
                panic!("Sell price data should be CEX for CEX-only scan");
            }
        }

        // Verify sorting (each opportunity should have profit_percentage >= next one)
        if i < opportunities.len() - 1 {
            assert!(
                opp.profit_percentage >= opportunities[i + 1].profit_percentage,
                "Opportunities should be sorted by profit percentage (descending) - Opportunity #{} has {:.4}% but #{} has {:.4}%",
                i + 1,
                opp.profit_percentage,
                i + 2,
                opportunities[i + 1].profit_percentage
            );
        }
    }

    // Show top opportunities summary
    if opportunities.len() > 1 {
        println!("\n=== Top 10 Most Profitable Opportunities ===");
        for (i, opp) in opportunities.iter().take(10).enumerate() {
            println!(
                "  #{}: {} -> {} | Profit: {:.4}% | ${:.4}",
                i + 1,
                opp.buy_exchange,
                opp.sell_exchange,
                opp.profit_percentage,
                opp.profit
            );
        }
    }

    println!(
        "\n✓ CEX arbitrage scan test passed for {} (all {} exchanges tested)\n",
        TEST_SYMBOL,
        cex_exchanges.len()
    );
}
