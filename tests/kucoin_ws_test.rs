//! KuCoin WebSocket test: spotMarket level1, multi-symbol.
//! Run: cargo test kucoin_ws_test -- --nocapture

use aeon_market_scanner_rs::{CEXTrait, Kucoin};

#[tokio::test]
async fn kucoin_ws_stream_multi_symbol() {
    println!("\n=== KuCoin WebSocket level1 – multi-symbol (BTCUSDT, ETHUSDT) ===\n");

    let exchange = Kucoin::new();
    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 5, 5000)
        .await
        .expect("KuCoin WebSocket stream");

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
                if seen.len() >= 2 && count >= 10 {
                    break;
                }
            }
        }
    }

    println!("\nReceived {} prices.", count);
}
