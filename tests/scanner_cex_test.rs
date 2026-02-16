mod scanner_common;
use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use scanner_common::{TEST_SYMBOL, get_all_cex_exchanges};

#[tokio::test]
async fn test_scan_cex_arbitrage_ethusdt() {
    println!("===== Testing CEX Arbitrage Scanner for ETHUSDT =====\n");

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
        println!("  Source (buy): {}", opp.source_exchange);
        println!("  Destination (sell): {}", opp.destination_exchange);
        println!("  Symbol: {}", opp.symbol);
        println!("  Effective ask: ${:.4}", opp.effective_ask);
        println!("  Effective bid: ${:.4}", opp.effective_bid);
        println!("  Spread: ${:.4}", opp.spread);
        println!("  Spread %: {:.4}%", opp.spread_percentage);
        println!("  Executable quantity: {:.4}", opp.executable_quantity);
        println!("  Total profit: ${:.4}", opp.total_profit());
        println!(
            "  Source commission: {:.4}% | Dest commission: {:.4}% | Total commission (USD): ${:.4}",
            opp.source_commission_percent,
            opp.destination_commission_percent,
            opp.total_commission_quote
        );

        // Show full price data from buy and sell responses
        println!("  Source Leg:");
        match &opp.source_leg {
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
        println!("  Destination Leg:");
        match &opp.destination_leg {
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
        assert!(opp.spread > 0.0, "Spread should be positive");
        assert!(
            opp.spread_percentage > 0.0,
            "Spread percentage should be positive"
        );
        assert!(opp.effective_ask > 0.0, "Effective ask should be positive");
        assert!(opp.effective_bid > 0.0, "Effective bid should be positive");
        assert!(
            opp.effective_bid > opp.effective_ask,
            "Effective bid should be higher than effective ask"
        );

        // Verify price data is present and contains full response
        match &opp.source_leg {
            PriceData::Cex(cex_price) => {
                assert_eq!(cex_price.symbol, TEST_SYMBOL);
                assert!(
                    opp.effective_ask >= cex_price.ask_price,
                    "Effective ask (with fee) >= raw ask"
                );
                assert!(cex_price.timestamp > 0, "Timestamp should be present");
                assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            }
            PriceData::Dex(_) => {
                panic!("Source leg should be CEX for CEX-only scan");
            }
        }

        match &opp.destination_leg {
            PriceData::Cex(cex_price) => {
                assert_eq!(cex_price.symbol, TEST_SYMBOL);
                assert!(
                    opp.effective_bid <= cex_price.bid_price,
                    "Effective bid (with fee) <= raw bid"
                );
                assert!(cex_price.timestamp > 0, "Timestamp should be present");
                assert!(cex_price.mid_price > 0.0, "Mid price should be present");
            }
            PriceData::Dex(_) => {
                panic!("Destination leg should be CEX for CEX-only scan");
            }
        }

        // Verify sorting (each opportunity should have profit_percentage >= next one)
        if i < opportunities.len() - 1 {
            assert!(
                opp.spread_percentage >= opportunities[i + 1].spread_percentage,
                "Opportunities should be sorted by spread percentage (descending) - Opportunity #{} has {:.4}% but #{} has {:.4}%",
                i + 1,
                opp.spread_percentage,
                i + 2,
                opportunities[i + 1].spread_percentage
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
                opp.source_exchange,
                opp.destination_exchange,
                opp.spread_percentage,
                opp.spread
            );
        }
    }

    println!(
        "\n✓ CEX arbitrage scan test passed for {} (all {} exchanges tested)\n",
        TEST_SYMBOL,
        cex_exchanges.len()
    );
}
