//! Kraken WebSocket test: stream book channel, receive 5 prices and print.
//! Run: cargo test kraken_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Kraken};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn kraken_ws_stream_five_then_stop() {
    println!("\n=== Kraken WebSocket stream (book) – 5 prices then stop ===\n");

    let exchange = Kraken::new();
    let mut rx = exchange
        .stream_price_websocket(SYMBOL)
        .await
        .expect("Kraken WebSocket stream");

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
        count += 1;
        if count >= 5 {
            break;
        }
    }
    println!("\nReceived {} prices.", count);
}
