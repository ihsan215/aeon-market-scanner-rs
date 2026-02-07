//! Gate.io WebSocket test: stream depth via v3 (depth.subscribe), receive prices with bid/ask qty.
//! Run: cargo test gateio_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Gateio};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn gateio_ws_stream_ten_then_stop() {
    println!("\n=== Gate.io WebSocket v3 (depth.subscribe) – 5 prices then stop ===\n");

    let exchange = Gateio::new();
    let mut rx = exchange
        .stream_price_websocket(SYMBOL)
        .await
        .expect("Gate.io WebSocket stream");

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
