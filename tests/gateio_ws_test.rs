//! Gate.io WebSocket test: stream depth via v3 (depth.subscribe), receive prices with bid/ask qty.
//! Run: cargo test gateio_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Gateio};

#[tokio::test]
async fn gateio_ws_stream_multi_symbol() {
    println!(
        "\n=== Gate.io WebSocket v3 (depth.subscribe) – multi-symbol (BTCUSDT, ETHUSDT) ===\n"
    );

    let exchange = Gateio::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 5, 5000)
        .await
        .expect("Gate.io WebSocket stream");

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
