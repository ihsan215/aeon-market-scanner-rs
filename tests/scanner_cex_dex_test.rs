mod scanner_common;

use aeon_market_scanner_rs::DexAggregator;

use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use scanner_common::{
    QUOTE_AMOUNT, TEST_SYMBOL, create_eth_eth, create_eth_usdt, get_all_cex_exchanges,
};

#[tokio::test]
async fn test_scan_cex_dex_arbitrage_ethusdt() {
    println!("===== Testing CEX-DEX Arbitrage Scanner for ETHUSDT =====\n");

    let cex_exchanges = get_all_cex_exchanges();
    let dex_exchanges = vec![DexAggregator::KyberSwap];

    println!(
        "Scanning {} CEX exchanges and {} DEX exchanges for {} arbitrage opportunities...\n",
        cex_exchanges.len(),
        dex_exchanges.len(),
        TEST_SYMBOL
    );

    // Create Ethereum tokens for DEX (KyberSwap)
    let eth_token = create_eth_eth();
    let usdt_token = create_eth_usdt();

    let result = ArbitrageScanner::scan_arbitrage_opportunities(
        TEST_SYMBOL,
        &cex_exchanges,
        Some(&dex_exchanges),
        Some(&eth_token),
        Some(&usdt_token),
        Some(QUOTE_AMOUNT),
    )
    .await;

    assert!(
        result.is_ok(),
        "Should successfully scan CEX-DEX arbitrage opportunities: {:?}",
        result.err()
    );

    let opportunities = result.unwrap();
    println!(
        "Found {} arbitrage opportunities for {}\n",
        opportunities.len(),
        TEST_SYMBOL
    );

    if opportunities.is_empty() {
        println!("No arbitrage opportunities found");
        return;
    }

    let mut cex_cex_count = 0;
    let mut cex_dex_count = 0;
    let mut dex_cex_count = 0;
    let mut dex_dex_count = 0;

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
                println!("    Type: CEX");
                println!("    Exchange: {:?}", cex_price.exchange);
                println!("    Symbol: {}", cex_price.symbol);
                println!("    Timestamp: {}", cex_price.timestamp);
                println!("    Bid Price: ${:.4}", cex_price.bid_price);
                println!("    Ask Price: ${:.4}", cex_price.ask_price);
                println!("    Mid Price: ${:.4}", cex_price.mid_price);
                println!("    Bid Quantity: {:.4}", cex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", cex_price.ask_qty);
            }
            PriceData::Dex(dex_price) => {
                println!("    Type: DEX");
                println!("    Exchange: {:?}", dex_price.exchange);
                println!("    Symbol: {}", dex_price.symbol);
                println!("    Timestamp: {}", dex_price.timestamp);
                println!("    Bid Price: ${:.4}", dex_price.bid_price);
                println!("    Ask Price: ${:.4}", dex_price.ask_price);
                println!("    Mid Price: ${:.4}", dex_price.mid_price);
                println!("    Bid Quantity: {:.4}", dex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", dex_price.ask_qty);
                if let Some(ref route) = dex_price.ask_route_summary {
                    println!("    Ask Route Summary:");
                    println!("      Token In: {}", route.token_in);
                    println!("      Token Out: {}", route.token_out);
                    println!("      Amount In: {}", route.amount_in);
                    println!("      Amount Out: {}", route.amount_out);
                    println!("      Amount In Wei: {:.4}", route.amount_in_wei);
                    println!("      Amount Out Wei: {:.4}", route.amount_out_wei);
                }
                if dex_price.ask_route_data.is_some() {
                    println!("    Ask Route Data: Available (JSON)");
                }
            }
        }
        println!("  Destination Leg:");
        match &opp.destination_leg {
            PriceData::Cex(cex_price) => {
                println!("    Type: CEX");
                println!("    Exchange: {:?}", cex_price.exchange);
                println!("    Symbol: {}", cex_price.symbol);
                println!("    Timestamp: {}", cex_price.timestamp);
                println!("    Bid Price: ${:.4}", cex_price.bid_price);
                println!("    Ask Price: ${:.4}", cex_price.ask_price);
                println!("    Mid Price: ${:.4}", cex_price.mid_price);
                println!("    Bid Quantity: {:.4}", cex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", cex_price.ask_qty);
            }
            PriceData::Dex(dex_price) => {
                println!("    Type: DEX");
                println!("    Exchange: {:?}", dex_price.exchange);
                println!("    Symbol: {}", dex_price.symbol);
                println!("    Timestamp: {}", dex_price.timestamp);
                println!("    Bid Price: ${:.4}", dex_price.bid_price);
                println!("    Ask Price: ${:.4}", dex_price.ask_price);
                println!("    Mid Price: ${:.4}", dex_price.mid_price);
                println!("    Bid Quantity: {:.4}", dex_price.bid_qty);
                println!("    Ask Quantity: {:.4}", dex_price.ask_qty);
                if let Some(ref route) = dex_price.bid_route_summary {
                    println!("    Bid Route Summary:");
                    println!("      Token In: {}", route.token_in);
                    println!("      Token Out: {}", route.token_out);
                    println!("      Amount In: {}", route.amount_in);
                    println!("      Amount Out: {}", route.amount_out);
                    println!("      Amount In Wei: {:.4}", route.amount_in_wei);
                    println!("      Amount Out Wei: {:.4}", route.amount_out_wei);
                }
                if dex_price.bid_route_data.is_some() {
                    println!("    Bid Route Data: Available (JSON)");
                }
            }
        }
        println!();

        // Verify profit is positive
        assert!(opp.spread > 0.0, "Spread should be positive");
        assert!(
            opp.spread_percentage > 0.0,
            "Spread percentage should be positive"
        );
        assert!(
            opp.effective_bid > opp.effective_ask,
            "Effective bid should be higher than effective ask"
        );

        // Categorize opportunities
        match (&opp.source_leg, &opp.destination_leg) {
            (PriceData::Cex(_), PriceData::Cex(_)) => {
                cex_cex_count += 1;
            }
            (PriceData::Cex(_), PriceData::Dex(_)) => {
                cex_dex_count += 1;
                // Verify DEX route data is present
                if let PriceData::Dex(dex_price) = &opp.destination_leg {
                    assert!(
                        dex_price.bid_route_summary.is_some() || dex_price.bid_route_data.is_some(),
                        "DEX sell should have route data"
                    );
                }
            }
            (PriceData::Dex(_), PriceData::Cex(_)) => {
                dex_cex_count += 1;
                // Verify DEX route data is present
                if let PriceData::Dex(dex_price) = &opp.source_leg {
                    assert!(
                        dex_price.ask_route_summary.is_some() || dex_price.ask_route_data.is_some(),
                        "DEX buy should have route data"
                    );
                }
            }
            (PriceData::Dex(_), PriceData::Dex(_)) => {
                dex_dex_count += 1;
                // Verify DEX route data is present for both
                if let PriceData::Dex(buy_dex) = &opp.source_leg {
                    assert!(
                        buy_dex.ask_route_summary.is_some() || buy_dex.ask_route_data.is_some(),
                        "DEX buy should have route data"
                    );
                }
                if let PriceData::Dex(sell_dex) = &opp.destination_leg {
                    assert!(
                        sell_dex.bid_route_summary.is_some() || sell_dex.bid_route_data.is_some(),
                        "DEX sell should have route data"
                    );
                }
            }
        }

        // Verify sorting (most profitable first)
        if i < opportunities.len() - 1 {
            assert!(
                opp.spread_percentage >= opportunities[i + 1].spread_percentage,
                "Opportunities should be sorted by spread percentage (descending)"
            );
        }
    }

    println!("\n=== Opportunity Breakdown ===");
    println!("  CEX-CEX: {}", cex_cex_count);
    println!("  CEX-DEX: {}", cex_dex_count);
    println!("  DEX-CEX: {}", dex_cex_count);
    println!("  DEX-DEX: {}", dex_dex_count);

    // Show top opportunities
    if opportunities.len() > 1 {
        println!(
            "\n=== Top {} Most Profitable Opportunities ===",
            opportunities.len()
        );
        for (i, opp) in opportunities.iter().enumerate() {
            let opp_type = match (&opp.source_leg, &opp.destination_leg) {
                (PriceData::Cex(_), PriceData::Cex(_)) => "CEX-CEX",
                (PriceData::Cex(_), PriceData::Dex(_)) => "CEX-DEX",
                (PriceData::Dex(_), PriceData::Cex(_)) => "DEX-CEX",
                (PriceData::Dex(_), PriceData::Dex(_)) => "DEX-DEX",
            };
            println!(
                "  #{}: {} -> {} | Profit: {:.4}% | ${:.4} | Type: {}",
                i + 1,
                opp.source_exchange,
                opp.destination_exchange,
                opp.spread_percentage,
                opp.spread,
                opp_type
            );
        }
    }

    println!(
        "\n✓ CEX-DEX arbitrage scan test passed for {} (all {} CEX exchanges tested)\n",
        TEST_SYMBOL,
        cex_exchanges.len()
    );
}
