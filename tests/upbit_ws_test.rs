//! Upbit WebSocket test: orderbook stream, multi-symbol.
//! Run: cargo test upbit_ws -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Upbit};

#[tokio::test]
async fn upbit_ws_stream_multi_symbol() {
    println!("\n=== Upbit WebSocket orderbook – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Upbit::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], true, None)
        .await
        .expect("Upbit WebSocket stream");

    let mut count = 0u32;
    let mut seen = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(25);
    loop {
        tokio::select! {
            _ = tokio::time::sleep_until(deadline) => break,
            msg = rx.recv() => {
                let price = match msg {
                    Some(p) => p,
                    None => break,
                };
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
        }
    }
    println!("\nReceived {} prices.", count);
}
