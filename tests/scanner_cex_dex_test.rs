mod scanner_common;

use aeon_market_scanner_rs::DexAggregator;

use aeon_market_scanner_rs::scanner::{ArbitrageScanner, PriceData};
use scanner_common::{
    QUOTE_AMOUNT, TEST_SYMBOL, create_bsc_bnb, create_bsc_usdt, get_all_cex_exchanges,
};

#[tokio::test]
async fn test_scan_cex_dex_arbitrage_bnbusdt() {
    println!("===== Testing CEX-DEX Arbitrage Scanner for BNBUSDT =====\n");

    let cex_exchanges = get_all_cex_exchanges();
    let dex_exchanges = vec![DexAggregator::KyberSwap];

    println!(
        "Scanning {} CEX exchanges and {} DEX exchanges for {} arbitrage opportunities...\n",
        cex_exchanges.len(),
        dex_exchanges.len(),
        TEST_SYMBOL
    );

    // Create BSC tokens algorithmically (no hard-coded token provider)
    let bnb_token = create_bsc_bnb();
    let usdt_token = create_bsc_usdt();

    let result = ArbitrageScanner::scan_arbitrage_opportunities(
        TEST_SYMBOL,
        &cex_exchanges,
        Some(&dex_exchanges),
        Some(&bnb_token),
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
        println!("  Buy from: {}", opp.buy_exchange);
        println!("  Sell on: {}", opp.sell_exchange);
        println!("  Symbol: {}", opp.symbol);
        println!("  Buy price: ${:.4}", opp.buy_price);
        println!("  Sell price: ${:.4}", opp.sell_price);
        println!("  Profit: ${:.4}", opp.profit);
        println!("  Profit %: {:.4}%", opp.profit_percentage);
        println!("  Total profit: ${:.4}", opp.total_profit());

        // Show full price data from buy and sell responses
        println!("  Buy Price Data:");
        match &opp.buy_price_data {
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
        println!("  Sell Price Data:");
        match &opp.sell_price_data {
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
        assert!(opp.profit > 0.0, "Profit should be positive");
        assert!(
            opp.profit_percentage > 0.0,
            "Profit percentage should be positive"
        );
        assert!(
            opp.sell_price > opp.buy_price,
            "Sell price should be higher than buy price"
        );

        // Categorize opportunities
        match (&opp.buy_price_data, &opp.sell_price_data) {
            (PriceData::Cex(_), PriceData::Cex(_)) => {
                cex_cex_count += 1;
            }
            (PriceData::Cex(_), PriceData::Dex(_)) => {
                cex_dex_count += 1;
                // Verify DEX route data is present
                if let PriceData::Dex(dex_price) = &opp.sell_price_data {
                    assert!(
                        dex_price.bid_route_summary.is_some() || dex_price.bid_route_data.is_some(),
                        "DEX sell should have route data"
                    );
                }
            }
            (PriceData::Dex(_), PriceData::Cex(_)) => {
                dex_cex_count += 1;
                // Verify DEX route data is present
                if let PriceData::Dex(dex_price) = &opp.buy_price_data {
                    assert!(
                        dex_price.ask_route_summary.is_some() || dex_price.ask_route_data.is_some(),
                        "DEX buy should have route data"
                    );
                }
            }
            (PriceData::Dex(_), PriceData::Dex(_)) => {
                dex_dex_count += 1;
                // Verify DEX route data is present for both
                if let PriceData::Dex(buy_dex) = &opp.buy_price_data {
                    assert!(
                        buy_dex.ask_route_summary.is_some() || buy_dex.ask_route_data.is_some(),
                        "DEX buy should have route data"
                    );
                }
                if let PriceData::Dex(sell_dex) = &opp.sell_price_data {
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
                opp.profit_percentage >= opportunities[i + 1].profit_percentage,
                "Opportunities should be sorted by profit percentage (descending)"
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
            let opp_type = match (&opp.buy_price_data, &opp.sell_price_data) {
                (PriceData::Cex(_), PriceData::Cex(_)) => "CEX-CEX",
                (PriceData::Cex(_), PriceData::Dex(_)) => "CEX-DEX",
                (PriceData::Dex(_), PriceData::Cex(_)) => "DEX-CEX",
                (PriceData::Dex(_), PriceData::Dex(_)) => "DEX-DEX",
            };
            println!(
                "  #{}: {} -> {} | Profit: {:.4}% | ${:.4} | Type: {}",
                i + 1,
                opp.buy_exchange,
                opp.sell_exchange,
                opp.profit_percentage,
                opp.profit,
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
