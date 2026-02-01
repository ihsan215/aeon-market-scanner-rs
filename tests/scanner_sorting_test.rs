mod scanner_common;
use aeon_market_scanner_rs::scanner::ArbitrageScanner;
use scanner_common::{TEST_SYMBOL, get_all_cex_exchanges};

#[tokio::test]
async fn test_arbitrage_sorting_verification_bnbusdt() {
    println!("===== Testing Arbitrage Sorting Verification for BNBUSDT =====\n");

    let cex_exchanges = get_all_cex_exchanges();

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

    if opportunities.len() < 2 {
        println!(
            "Not enough opportunities to verify sorting (found {})",
            opportunities.len()
        );
        return;
    }

    println!(
        "Verifying that {} opportunities are sorted from most profitable to least profitable...\n",
        opportunities.len()
    );

    // Verify sorting: each opportunity should have profit_percentage >= next one
    for i in 0..opportunities.len() - 1 {
        let current = &opportunities[i];
        let next = &opportunities[i + 1];

        assert!(
            current.spread_percentage >= next.spread_percentage,
            "Opportunity #{} ({:.4}%) should be >= Opportunity #{} ({:.4}%)",
            i + 1,
            current.spread_percentage,
            i + 2,
            next.spread_percentage
        );
    }

    println!("\nTop 5 most profitable opportunities for {}:", TEST_SYMBOL);
    for (i, opp) in opportunities.iter().take(5).enumerate() {
        println!(
            "  #{}: {} -> {} | {:.4}% | ${:.4}",
            i + 1,
            opp.source_exchange,
            opp.destination_exchange,
            opp.spread_percentage,
            opp.spread
        );
    }

    println!(
        "\n✓ Sorting verification passed - all opportunities are sorted from most profitable to least profitable\n"
    );
}
