//! Bybit WebSocket test: stream orderbook.1, receive 10 prices and print.
//! Run: cargo test bybit_ws -- --nocapture

use aeon_market_scanner_rs::{Bybit, CEXTrait};

#[tokio::test]
async fn bybit_ws_stream_multi_symbol() {
    println!("\n=== Bybit WebSocket stream (orderbook.1) – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Bybit::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], true, None)
        .await
        .expect("Bybit WebSocket stream");

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
