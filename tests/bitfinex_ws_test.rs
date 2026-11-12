//! Bitfinex WebSocket test: continuous feed, print 10 prices then stop.
//! Run: cargo test bitfinex_ws -- --nocapture

use aeon_market_scanner_rs::{Bitfinex, CEXTrait};

#[tokio::test]
async fn bitfinex_ws_stream_multi_symbol() {
    println!("\n=== Bitfinex WebSocket stream – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Bitfinex::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 5, 5000)
        .await
        .expect("WebSocket stream");

    let mut count = 0u32;
    let mut seen = std::collections::HashSet::new();
    while let Some(price) = rx.recv().await {
        println!(
            "{}  bid: {:>12}  ask: {:>12}  mid: {:>12}  (bid_qty: {}, ask_qty: {})",
            price.symbol,
            price.bid_price,
            price.ask_price,
            price.mid_price,
            price.bid_qty,
            price.ask_qty
        );
        seen.insert(price.symbol.clone());
        count += 1;
        if seen.len() >= 2 && count >= 10 {
            break;
        }
    }
    assert!(
        seen.len() >= 2,
        "Expected both BTCUSDT and ETHUSDT; got {:?}",
        seen
    );
    println!("\nReceived {} prices from {:?}.", count, seen);
}

/// Regression (0.8.0): WS prices used the raw channel symbol (`TBTCUST`, `TDOGE:UST`).
/// (Colon-style names like `tDOGE:UST` are covered by unit tests in `src/common/utils.rs`.)
#[tokio::test]
async fn bitfinex_ws_reports_standard_symbols() {
    let expected = ["BTCUSDT", "ETHUSDT", "SOLUSDT"];
    let mut rx = Bitfinex::new()
        .stream_price_websocket(&expected, 5, 5000)
        .await
        .expect("WebSocket stream");

    let mut seen = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while seen.len() < expected.len() {
        let price = tokio::time::timeout_at(deadline, rx.recv())
            .await
            .expect("timed out waiting for all symbols")
            .expect("stream closed");
        assert!(
            expected.contains(&price.symbol.as_str()),
            "unexpected symbol {:?} (should be standard BTCUSDT form)",
            price.symbol
        );
        seen.insert(price.symbol);
    }
    println!("Bitfinex WS symbols: {:?}", seen);
}
