//! MEXC WebSocket test: bookTicker stream (protobuf), multi-symbol.
//! Run: cargo test mexc_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Mexc};

#[tokio::test]
async fn mexc_ws_stream_multi_symbol() {
    println!("\n=== MEXC WebSocket bookTicker (protobuf) – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Mexc::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], true, None)
        .await
        .expect("MEXC WebSocket stream");

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
        if seen.len() >= 2 && count >= 20 {
            break;
        }
    }
    println!("\nReceived {} prices.", count);
}
