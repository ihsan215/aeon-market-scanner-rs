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

/// Regression (0.8.0): depth was subscribed with interval "0.01", which merged price levels
/// to 0.01 steps, so DOGE (~0.07) came back as bid 0.06 / ask 0.07 (~15% spread).
#[tokio::test]
async fn gateio_ws_low_priced_assets_are_not_rounded() {
    let mut rx = Gateio::new()
        .stream_price_websocket(&["DOGEUSDT", "PEPEUSDT"], 5, 5000)
        .await
        .expect("Gate.io WebSocket stream");

    let mut checked = 0;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while checked < 6 {
        let p = tokio::time::timeout_at(deadline, rx.recv())
            .await
            .expect("timed out waiting for Gate prices")
            .expect("stream closed");
        let spread = (p.ask_price - p.bid_price) / p.mid_price;
        println!(
            "{} bid {} ask {} spread {:.4}%",
            p.symbol,
            p.bid_price,
            p.ask_price,
            spread * 100.0
        );
        assert!(p.bid_price > 0.0 && p.ask_price >= p.bid_price);
        assert!(
            spread < 0.01,
            "{} spread {:.2}% looks price-merged (bid {}, ask {})",
            p.symbol,
            spread * 100.0,
            p.bid_price,
            p.ask_price
        );
        checked += 1;
    }
}
