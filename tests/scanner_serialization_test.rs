mod scanner_common;
use aeon_market_scanner_rs::CexExchange;
use aeon_market_scanner_rs::scanner::{ArbitrageOpportunity, ArbitrageScanner, PriceData};
use scanner_common::TEST_SYMBOL;

#[tokio::test]
async fn test_arbitrage_serialization_ethusdt() {
    println!("===== Testing ArbitrageOpportunity Serialization for ETHUSDT =====\n");

    let cex_exchanges = vec![CexExchange::Binance, CexExchange::Bybit, CexExchange::OKX];

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

    assert!(result.is_ok(), "Should successfully scan arbitrage");

    let opportunities = result.unwrap();

    if opportunities.is_empty() {
        println!("No arbitrage opportunities found (skipping serialization test)");
        return;
    }

    // Test JSON serialization with the most profitable opportunity
    let opp = &opportunities[0];
    let json_result = serde_json::to_string(opp);

    assert!(
        json_result.is_ok(),
        "Should be able to serialize ArbitrageOpportunity to JSON: {:?}",
        json_result.err()
    );

    let json_string = json_result.unwrap();
    println!("Serialized {} opportunity (first 500 chars):", TEST_SYMBOL);
    println!("{}...", &json_string[..json_string.len().min(500)]);

    // Test JSON deserialization
    let deserialized_result: Result<ArbitrageOpportunity, _> = serde_json::from_str(&json_string);

    assert!(
        deserialized_result.is_ok(),
        "Should be able to deserialize ArbitrageOpportunity from JSON: {:?}",
        deserialized_result.err()
    );

    let deserialized = deserialized_result.unwrap();
    assert_eq!(deserialized.symbol, opp.symbol);
    assert_eq!(deserialized.source_exchange, opp.source_exchange);
    assert_eq!(deserialized.destination_exchange, opp.destination_exchange);
    assert!((deserialized.effective_ask - opp.effective_ask).abs() < 0.0001);
    assert!((deserialized.effective_bid - opp.effective_bid).abs() < 0.0001);

    // Verify that response data is preserved in serialization
    match (&deserialized.source_leg, &opp.source_leg) {
        (PriceData::Cex(deserialized_cex), PriceData::Cex(original_cex)) => {
            assert_eq!(deserialized_cex.timestamp, original_cex.timestamp);
            assert_eq!(deserialized_cex.mid_price, original_cex.mid_price);
            println!("  ✓ Buy response data (timestamp, mid_price) preserved in serialization");
        }
        _ => {}
    }

    println!("✓ Serialization test passed for {}\n", TEST_SYMBOL);
}
