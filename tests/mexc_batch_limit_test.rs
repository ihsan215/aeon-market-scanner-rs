//! MEXC spot **batch limit (IOC)** on `XRPUSDT`: in one request, place **buy @ ask** and **sell @ bid**
//! (IOC) from the book. The private order **WebSocket is opened first** (before book snapshot and
//! batch submit), then REST prints the batch response; WS lines are printed **only** for updates
//! that match those REST `order_id`s (execution path).
//!
//! Run: `cargo test mexc_batch_limit_xrp_buy_sell_with_ws -- --nocapture`
//!
//! Requires `MEXC_API_KEY` / `MEXC_API_SECRET`. The sell leg needs enough XRP on spot (same idea as
//! the market batch test). If the book is thin, IOC orders may partially fill then end in
//! `PartiallyCancelled`; the test waits on the WS for a **terminal** status (`Filled` / `Cancelled` / `PartiallyCancelled`).

use aeon_market_scanner_rs::cex::mexc::{
    Mexc, MexcBatchOrderResult, MexcLimitOrderType, MexcOrderStatus, MexcOrderType, MexcTradeSide,
};
use aeon_market_scanner_rs::CEXTrait;
use std::collections::HashSet;
use std::time::Duration;

fn has_mexc_credentials() -> bool {
    let _ = dotenvy::dotenv();

    matches!(std::env::var("MEXC_API_KEY"), Ok(value) if !value.is_empty())
        && matches!(std::env::var("MEXC_API_SECRET"), Ok(value) if !value.is_empty())
}

fn batch_symbol() -> String {
    std::env::var("MEXC_BATCH_LIMIT_SYMBOL").unwrap_or_else(|_| "XRPUSDT".to_string())
}

fn base_qty_str() -> String {
    std::env::var("MEXC_BATCH_LIMIT_BASE_QTY").unwrap_or_else(|_| "1".to_string())
}

fn format_price(p: f64) -> String {
    let s = format!("{:.8}", p);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[tokio::test]
async fn mexc_batch_limit_xrp_buy_sell_with_ws() {
    if !has_mexc_credentials() {
        println!("Skipping: set MEXC_API_KEY and MEXC_API_SECRET");
        return;
    }

    let symbol = batch_symbol();
    let qty = base_qty_str();

    let mexc = Mexc::with_credentials(
        std::env::var("MEXC_API_KEY").unwrap(),
        std::env::var("MEXC_API_SECRET").unwrap(),
    );

    println!(
        "\n=== MEXC batch LIMIT (IOC): {} — buy @ ask + sell @ bid, qty {} ===\n",
        symbol, qty
    );

    // Subscribe before book + batch so execution pushes are not missed.
    let mut rx = mexc
        .stream_spot_order_updates(5, 5_000)
        .await
        .expect("MEXC private order stream");
    println!("WebSocket: private order stream connected (listening before REST book + batch).\n");

    let book = mexc
        .get_price(&symbol)
        .await
        .expect("get_price (book ticker)");
    let bid = book.bid_price;
    let ask = book.ask_price;
    assert!(bid > 0.0 && ask > 0.0 && ask >= bid, "invalid book bid={bid} ask={ask}");
    let ask_str = format_price(ask);
    let bid_str = format_price(bid);
    println!(
        "Book: bid={} ask={} spread={} → IOC BUY @ {} | IOC SELL @ {}",
        bid,
        ask,
        ask - bid,
        ask_str,
        bid_str
    );

    let legs = [
        (
            MexcTradeSide::Buy,
            ask_str.as_str(),
            qty.as_str(),
            MexcLimitOrderType::ImmediateOrCancel,
        ),
        (
            MexcTradeSide::Sell,
            bid_str.as_str(),
            qty.as_str(),
            MexcLimitOrderType::ImmediateOrCancel,
        ),
    ];

    println!("Placing batch IOC orders (REST)…");
    let batch_results: Vec<MexcBatchOrderResult> = mexc
        .place_batch_limit_orders(&symbol, &legs)
        .await
        .expect("place_batch_limit_orders");

    println!("--- REST batchOrders response ({} rows) ---", batch_results.len());
    for (i, row) in batch_results.iter().enumerate() {
        println!("  [{}] {:?}", i, row);
    }

    let mut pending: HashSet<String> = batch_results
        .iter()
        .filter_map(|r| r.order_id.clone())
        .collect();

    if pending.is_empty() {
        println!("No order_id in REST response; nothing to match on WebSocket.");
        return;
    }

    println!(
        "\n--- WebSocket execution updates (only REST batch order_ids; terminal = Filled / PartiallyCancelled / Cancelled): {:?} ---\n",
        pending
    );

    let ws_timeout = Duration::from_secs(90);
    let deadline = tokio::time::Instant::now() + ws_timeout;

    loop {
        if pending.is_empty() {
            break;
        }
        if tokio::time::Instant::now() > deadline {
            panic!(
                "timeout waiting for WS terminal status; still pending order_id(s): {:?}",
                pending
            );
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let update = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for WS message")
            .expect("order stream closed");

        if !pending.contains(&update.order_id) {
            continue;
        }

        let d = update.to_display();
        println!(
            "[WS execution] order_id={} symbol={} side={} type={} status={} executed={} avg_price={} cum_qty={} send_time_ms={} display_time_ms={} channel={}",
            update.order_id,
            update.symbol,
            d.side,
            d.order_type,
            d.status,
            d.executed,
            d.average_filled_price,
            update.cumulative_quantity,
            update.send_time,
            d.time,
            update.channel
        );
        if !matches!(
            update.order_type,
            MexcOrderType::ImmediateOrCancel | MexcOrderType::Limit
        ) {
            continue;
        }

        let terminal = matches!(
            update.status,
            MexcOrderStatus::Filled
                | MexcOrderStatus::Cancelled
                | MexcOrderStatus::PartiallyCancelled
        );
        if terminal {
            pending.remove(&update.order_id);
        }
    }

    println!("\n=== Batch limit (IOC) test finished (each REST order_id reached a terminal WS status) ===\n");
}
