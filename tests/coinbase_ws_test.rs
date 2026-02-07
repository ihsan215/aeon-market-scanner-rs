//! Coinbase WebSocket test: stream ticker, receive 10 prices and print.
//! Run: cargo test coinbase_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Coinbase};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn coinbase_ws_stream_ten_then_stop() {
    println!("\n=== Coinbase WebSocket stream (ticker) – 2 prices then stop ===\n");

    let exchange = Coinbase::new();
    let mut rx = exchange
        .stream_price_websocket(SYMBOL)
        .await
        .expect("Coinbase WebSocket stream");

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
        if count >= 2 {
            break;
        }
    }
    println!("\nReceived {} prices.", count);
}
