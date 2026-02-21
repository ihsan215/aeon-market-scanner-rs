//! Kraken WebSocket test: stream book channel, receive 5 prices and print.
//! Run: cargo test kraken_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Kraken};

#[tokio::test]
async fn kraken_ws_stream_multi_symbol() {
    println!("\n=== Kraken WebSocket stream (book) – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Kraken::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 5, 5000)
        .await
        .expect("Kraken WebSocket stream");

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
    println!("\nReceived {} prices.", count);
}
