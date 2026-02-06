//! Bitfinex WebSocket test: connect via Bitfinex module, fetch one ticker, print, then disconnect.
//! Run: cargo test bitfinex_ws -- --nocapture

use aeon_market_scanner_rs::{Bitfinex, CEXTrait};

const SYMBOL: &str = "BTCUSDT";

#[tokio::test]
async fn bitfinex_ws_one_message_then_disconnect() {
    println!("\n=== Bitfinex WebSocket – one ticker, print, disconnect ===\n");

    let exchange = Bitfinex::new();
    let price = exchange
        .get_price_websocket(SYMBOL)
        .await
        .expect("WebSocket fetch");

    println!(
        "{}  bid: {:>12}  ask: {:>12}  mid: {:>12}  (bid_qty: {}, ask_qty: {})",
        price.symbol,
        price.bid_price,
        price.ask_price,
        price.mid_price,
        price.bid_qty,
        price.ask_qty
    );

    println!("\nConnection closed.");
}
