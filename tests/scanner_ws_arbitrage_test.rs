//! Arbitrage scanner WebSocket test: connects to WS streams and receives opportunity snapshots.
//! Run: cargo test scanner_ws_arbitrage -- --nocapture

use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange, FeeOverrides};

#[tokio::test]
async fn scan_arbitrage_from_websockets_basic() {
    println!("\n=== Arbitrage scanner from WebSocket streams ===\n");
    let fee_overrides = FeeOverrides::default()
        .with_cex_taker_fee(CexExchange::Binance, 0.000)
        .with_cex_taker_fee(CexExchange::OKX, 0.000)
        .with_cex_taker_fee(CexExchange::Bybit, 0.000);

    let mut rx = ArbitrageScanner::scan_arbitrage_from_websockets(
        &["BNBUSDT", "PEPEUSDT", "XRPUSDT", "ETHUSDT"],
        &[
            CexExchange::Binance,
            CexExchange::OKX,
            CexExchange::Bybit,
            CexExchange::Kucoin,
            CexExchange::Gateio,
            CexExchange::MEXC,
            CexExchange::Kraken,
            CexExchange::Bitfinex,
            CexExchange::Bitget,
        ],
        Some(&fee_overrides),
        true,
        Some(5),
    )
    .await
    .expect("scan_arbitrage_from_websockets");

    let mut snapshot_count = 0u32;
    let mut total_opps = 0u32;
    let mut top_spread: Option<f64> = None;
    let mut last_opps: Vec<aeon_market_scanner_rs::ArbitrageOpportunity> = Vec::new();

    let timeout = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        while let Some(opps) = rx.recv().await {
            snapshot_count += 1;
            total_opps += opps.len() as u32;

            if let Some(o) = opps.first() {
                top_spread = Some(o.spread_percentage);
                if snapshot_count <= 3 {
                    println!(
                        "Snapshot #{}: {} opps, top: {} -> {} {:.3}%",
                        snapshot_count,
                        opps.len(),
                        o.source_exchange,
                        o.destination_exchange,
                        o.spread_percentage
                    );
                }
            }
            if !opps.is_empty() {
                last_opps = opps;
            }
            if snapshot_count >= 100 {
                break;
            }
        }
    });

    match timeout.await {
        Ok(()) => {}
        Err(_) => println!("Timeout after 25s (OK if we received at least one snapshot)"),
    }

    println!(
        "\nReceived {} snapshots, {} total opportunities. Top spread: {:?}%",
        snapshot_count, total_opps, top_spread
    );

    println!("\n--- Opportunities (last snapshot, opportunity format) ---");
    for (i, o) in last_opps.iter().take(20).enumerate() {
        println!(
            "\n[{}] source_exchange: {}, destination_exchange: {}, symbol: {}",
            i + 1,
            o.source_exchange,
            o.destination_exchange,
            o.symbol
        );
        println!(
            "    effective_ask: {}, effective_bid: {}, spread: {}, spread_percentage: {}%",
            o.effective_ask, o.effective_bid, o.spread, o.spread_percentage
        );
        println!(
            "    executable_quantity: {}, total_commission_quote: {}, total_profit: {}",
            o.executable_quantity,
            o.total_commission_quote,
            o.total_profit()
        );
        println!(
            "    source_commission_percent: {}%, destination_commission_percent: {}%",
            o.source_commission_percent, o.destination_commission_percent
        );
    }
    if last_opps.len() > 20 {
        println!("\n... and {} more", last_opps.len() - 20);
    }

    assert!(
        snapshot_count >= 1,
        "Expected at least one opportunity snapshot; got {}",
        snapshot_count
    );
}
