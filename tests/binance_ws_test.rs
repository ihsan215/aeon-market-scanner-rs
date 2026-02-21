//! Binance WebSocket test: stream stays open; receive prices, print, then drop receiver.
//! Run: cargo test binance_ws -- --nocapture

use aeon_market_scanner_rs::{Binance, CEXTrait};

#[tokio::test]
async fn binance_ws_stream_multi_symbol() {
    println!("\n=== Binance WebSocket stream – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Binance::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 5, 5000)
        .await
        .expect("WebSocket stream");

    let mut seen = std::collections::HashSet::new();
    let mut count = 0u32;
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
