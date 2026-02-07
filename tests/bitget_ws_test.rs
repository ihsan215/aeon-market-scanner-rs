//! Bitget WebSocket test: continuous feed, print 10 prices then stop.
//! Run: cargo test bitget_ws -- --nocapture

use aeon_market_scanner_rs::{Bitget, CEXTrait};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn bitget_ws_stream_one_then_stop() {
    println!("\n=== Bitget WebSocket stream – continuous feed (stop after 10 prices) ===\n");

    let exchange = Bitget::new();
    let mut rx = exchange
        .stream_price_websocket(SYMBOL)
        .await
        .expect("WebSocket stream");

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
        if count >= 10 {
            break;
        }
    }
    println!("\nReceived {} prices, receiver dropped.", count);
}
