//! Binance WebSocket test: stream stays open; receive one price, print, then drop receiver.
//! Run: cargo test binance_ws -- --nocapture

use aeon_market_scanner_rs::{Binance, CEXTrait};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn binance_ws_stream_one_then_stop() {
    println!("\n=== Binance WebSocket stream – continuous feed (stop after 10 prices) ===\n");

    let exchange = Binance::new();
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
