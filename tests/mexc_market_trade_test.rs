//! MEXC spot market round-trip: market buy using `quoteOrderQty`, confirm fill on the private
//! order WebSocket, then market-sell the filled base amount.
//!
//! **Symbol:** MEXC lists USDC↔USDT as **`USDCUSDT`** (base **USDC**, quote **USDT**).
//! `quoteOrderQty` is in the **quote** asset (USDT). Default **2** USDT keeps the follow-up
//! sell above MEXC’s minimum notional on this pair.
//!
//! Run (live account, ~2 USDT quote spend plus fees):
//! `cargo test mexc_market_roundtrip -- --nocapture`
//!
//! Integration test loads keys from env and passes them via [`Mexc::with_credentials`].
//! Optional env:
//! - `MEXC_MARKET_TEST_SYMBOL` — default `USDCUSDT`.
//! - `MEXC_MARKET_TEST_QUOTE_USDT` — default `2`.

use aeon_market_scanner_rs::cex::mexc::{Mexc, MexcOrderStatus, MexcOrderType, MexcTradeSide};
use std::time::Duration;

fn has_mexc_credentials() -> bool {
    let _ = dotenvy::dotenv();

    matches!(std::env::var("MEXC_API_KEY"), Ok(value) if !value.is_empty())
        && matches!(std::env::var("MEXC_API_SECRET"), Ok(value) if !value.is_empty())
}

fn test_symbol() -> String {
    std::env::var("MEXC_MARKET_TEST_SYMBOL").unwrap_or_else(|_| "USDCUSDT".to_string())
}

fn quote_buy_usdt() -> String {
    std::env::var("MEXC_MARKET_TEST_QUOTE_USDT").unwrap_or_else(|_| "2".to_string())
}

/// Trim trailing zeros for MEXC quantity strings (step size still enforced by the API).
fn format_base_qty(qty: f64) -> String {
    let s = format!("{:.12}", qty);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[tokio::test]
async fn mexc_market_roundtrip_quote_buy_ws_then_sell() {
    if !has_mexc_credentials() {
        println!("Skipping: set MEXC_API_KEY and MEXC_API_SECRET");
        return;
    }

    let symbol = test_symbol();
    let quote_qty = quote_buy_usdt();
    let mexc = Mexc::with_credentials(
        std::env::var("MEXC_API_KEY").unwrap(),
        std::env::var("MEXC_API_SECRET").unwrap(),
    );

    let mut rx = mexc
        .stream_spot_order_updates(5, 5_000)
        .await
        .expect("MEXC private order stream");

    println!(
        "\n=== MEXC market test (USDC/USDT, symbol {}): {} USDT quote buy, then WS-confirmed market sell ===\n",
        symbol, quote_qty
    );

    let buy_placed = mexc
        .place_market_order_by_quote_amount(&symbol, MexcTradeSide::Buy, &quote_qty)
        .await
        .expect("market buy by quote");

    let buy_order_id = buy_placed.order_id.clone();
    println!(
        "Placed BUY market order_id={} symbol={} REST_transactTime_ms={}",
        buy_order_id, buy_placed.symbol, buy_placed.transact_time
    );

    let ws_timeout = Duration::from_secs(90);

    let deadline = tokio::time::Instant::now() + ws_timeout;
    let (filled_base, buy_fill_time_ms) = loop {
        if tokio::time::Instant::now() > deadline {
            panic!("timeout waiting for BUY fill on WebSocket (order_id={buy_order_id})");
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let update = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for WS message")
            .expect("order stream closed");

        if update.order_id != buy_order_id {
            continue;
        }
        if !matches!(update.order_type, MexcOrderType::Market) {
            continue;
        }
        if update.side != MexcTradeSide::Buy {
            continue;
        }

        let d = update.to_display();
        println!(
            "WS buy update: action={} status={} executed={} cumulative_qty={} | send_time_ms={} create_time_ms={} display_time_ms={}",
            d.event_action,
            d.status,
            d.executed,
            update.cumulative_quantity,
            update.send_time,
            update.create_time,
            d.time
        );

        if matches!(update.status, MexcOrderStatus::Filled) {
            break (update.filled_quantity(), d.time);
        }
    };

    assert!(
        filled_base > 0.0,
        "filled base amount must be positive, got {filled_base}"
    );

    let qty_str = format_base_qty(filled_base);
    println!(
        "Selling entire filled base qty={} (string={})",
        filled_base, qty_str
    );

    let sell_placed = mexc
        .place_market_order_by_quantity(&symbol, MexcTradeSide::Sell, &qty_str)
        .await
        .expect("market sell by quantity");

    let sell_order_id = sell_placed.order_id.clone();
    println!(
        "Placed SELL market order_id={} symbol={} REST_transactTime_ms={}",
        sell_order_id, sell_placed.symbol, sell_placed.transact_time
    );

    let deadline = tokio::time::Instant::now() + ws_timeout;
    loop {
        if tokio::time::Instant::now() > deadline {
            panic!("timeout waiting for SELL fill on WebSocket (order_id={sell_order_id})");
        }

        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        let update = tokio::time::timeout(remaining, rx.recv())
            .await
            .expect("timeout waiting for WS message")
            .expect("order stream closed");

        if update.order_id != sell_order_id {
            continue;
        }
        if !matches!(update.order_type, MexcOrderType::Market) {
            continue;
        }
        if update.side != MexcTradeSide::Sell {
            continue;
        }

        let d = update.to_display();
        println!(
            "WS sell update: action={} status={} executed={} | send_time_ms={} create_time_ms={} display_time_ms={}",
            d.event_action,
            d.status,
            d.executed,
            update.send_time,
            update.create_time,
            d.time
        );

        if matches!(update.status, MexcOrderStatus::Filled) {
            let sell_fill_time_ms = d.time;
            let delta_ws_ms = sell_fill_time_ms.saturating_sub(buy_fill_time_ms);
            let delta_rest_ms = sell_placed
                .transact_time
                .saturating_sub(buy_placed.transact_time);
            println!(
                "Round-trip complete: buy order_id={} filled base, sell filled.",
                buy_order_id
            );
            println!(
                "Timestamps (WS display_time_ms): buy_fill={} sell_fill={} Δ={} ms ({:.3} s)",
                buy_fill_time_ms,
                sell_fill_time_ms,
                delta_ws_ms,
                delta_ws_ms as f64 / 1000.0
            );
            println!(
                "Timestamps (REST transactTime_ms): buy={} sell={} Δ={} ms ({:.3} s)",
                buy_placed.transact_time,
                sell_placed.transact_time,
                delta_rest_ms,
                delta_rest_ms as f64 / 1000.0
            );
            return;
        }
    }
}
