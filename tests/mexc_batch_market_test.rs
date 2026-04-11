//! MEXC spot **batch market** on `XRPUSDT`: in one request, **buy** 1 XRP and **sell** 1 XRP,
//! listen on the private order WebSocket, and print the REST batch response plus matching WS lines.
//!
//! Run: `cargo test mexc_batch_market_xrp_buy_sell_with_ws -- --nocapture`
//!
//! Requires `MEXC_API_KEY` / `MEXC_API_SECRET`. The **sell** leg needs ~1 XRP in the spot wallet;
//! otherwise that leg may be rejected (REST `code` / `msg` and possibly no `order_id`).

use aeon_market_scanner_rs::cex::mexc::{
    Mexc, MexcBatchOrderItem, MexcBatchOrderResult, MexcOrderStatus, MexcOrderType, MexcTradeSide,
};
use std::collections::HashSet;
use std::time::Duration;

fn has_mexc_credentials() -> bool {
    let _ = dotenvy::dotenv();

    matches!(std::env::var("MEXC_API_KEY"), Ok(value) if !value.is_empty())
        && matches!(std::env::var("MEXC_API_SECRET"), Ok(value) if !value.is_empty())
}

fn batch_symbol() -> String {
    std::env::var("MEXC_BATCH_MARKET_SYMBOL").unwrap_or_else(|_| "XRPUSDT".to_string())
}

fn base_qty_str() -> String {
    std::env::var("MEXC_BATCH_MARKET_BASE_QTY").unwrap_or_else(|_| "1".to_string())
}

#[tokio::test]
async fn mexc_batch_market_xrp_buy_sell_with_ws() {
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
        "\n=== MEXC batch MARKET: {} — buy {} + sell {} (same request), WS listener ===\n",
        symbol, qty, qty
    );

    let mut rx = mexc
        .stream_spot_order_updates(5, 5_000)
        .await
        .expect("MEXC private order stream");

    let buy = MexcBatchOrderItem::market_by_quantity(&symbol, MexcTradeSide::Buy, &qty)
        .expect("batch buy leg");
    let sell = MexcBatchOrderItem::market_by_quantity(&symbol, MexcTradeSide::Sell, &qty)
        .expect("batch sell leg");

    let batch_results: Vec<MexcBatchOrderResult> = mexc
        .place_batch_orders(&[buy, sell])
        .await
        .expect("place_batch_orders");

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
        "\n--- WebSocket: waiting for Filled on order_id(s): {:?} ---\n",
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
                "timeout waiting for WS fills; still pending order_id(s): {:?}",
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
        if !matches!(update.order_type, MexcOrderType::Market) {
            continue;
        }

        let d = update.to_display();
        println!(
            "WS match: order_id={} side={} status={} executed={} avg_price={} | send_time_ms={} display_time_ms={} channel={}",
            update.order_id,
            d.side,
            d.status,
            d.executed,
            d.average_filled_price,
            update.send_time,
            d.time,
            update.channel
        );

        if matches!(update.status, MexcOrderStatus::Filled) {
            pending.remove(&update.order_id);
        }
    }

    println!("\n=== Batch market test finished (all reported REST order_ids saw WS Filled) ===\n");
}
