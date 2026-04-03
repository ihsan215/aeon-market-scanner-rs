//! MEXC spot IOC limit round-trip on `USDCUSDT`:
//! 1. Read book (`get_price` = bid/ask).
//! 2. IOC limit **buy** at **ask** (satış / offer) for ~2 USDT notional → should fill against the book.
//! 3. Wait for fill on the private order WebSocket.
//! 4. IOC limit **sell** at **bid** (alış) for the **same USDC** amount → should fill.
//!
//! Run: `cargo test mexc_limit_ioc_roundtrip -- --nocapture`
//!
//! Requires `MEXC_API_KEY` / `MEXC_API_SECRET`. Optional: `MEXC_MARKET_TEST_SYMBOL` (default `USDCUSDT`),
//! `MEXC_LIMIT_TEST_QUOTE_USDT` (default `2`).

use aeon_market_scanner_rs::cex::mexc::{
    Mexc, MexcLimitOrderType, MexcOrderStatus, MexcOrderType, MexcTradeSide,
};
use aeon_market_scanner_rs::CEXTrait;
use std::time::Duration;

fn has_mexc_credentials() -> bool {
    let _ = dotenvy::dotenv();

    matches!(std::env::var("MEXC_API_KEY"), Ok(value) if !value.is_empty())
        && matches!(std::env::var("MEXC_API_SECRET"), Ok(value) if !value.is_empty())
}

fn test_symbol() -> String {
    std::env::var("MEXC_MARKET_TEST_SYMBOL").unwrap_or_else(|_| "USDCUSDT".to_string())
}

fn quote_notional_usdt() -> f64 {
    std::env::var("MEXC_LIMIT_TEST_QUOTE_USDT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(2.0)
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
async fn mexc_limit_ioc_roundtrip() {
    if !has_mexc_credentials() {
        println!("Skipping: set MEXC_API_KEY and MEXC_API_SECRET");
        return;
    }

    let symbol = test_symbol();
    let quote_usdt = quote_notional_usdt();
    let mexc = Mexc::with_credentials(
        std::env::var("MEXC_API_KEY").unwrap(),
        std::env::var("MEXC_API_SECRET").unwrap(),
    );

    println!(
        "\n=== MEXC IOC limit test: {} — snapshot price, then IOC buy @ ask, IOC sell @ bid ===\n",
        symbol
    );

    let book = mexc
        .get_price(&symbol)
        .await
        .expect("get_price (book ticker)");
    let bid = book.bid_price;
    let ask = book.ask_price;
    assert!(bid > 0.0 && ask > 0.0 && ask >= bid, "invalid book bid={bid} ask={ask}");
    println!(
        "Book snapshot: bid={} ask={} spread={}",
        bid,
        ask,
        ask - bid
    );

    let base_qty_buy = quote_usdt / ask;
    let buy_qty_str = format_base_qty(base_qty_buy);
    let ask_price_str = format_price(ask);
    println!(
        "~{quote_usdt} USDT notional → buy qty (USDC)={} @ ask price={}",
        buy_qty_str, ask_price_str
    );

    let mut rx = mexc
        .stream_spot_order_updates(5, 5_000)
        .await
        .expect("MEXC private order stream");

    let buy_placed = mexc
        .place_limit_order(
            &symbol,
            MexcTradeSide::Buy,
            &ask_price_str,
            &buy_qty_str,
            MexcLimitOrderType::ImmediateOrCancel,
        )
        .await
        .expect("IOC limit buy");

    let buy_order_id = buy_placed.order_id.clone();
    println!(
        "Placed IOC BUY order_id={} price={} qty={} REST_transactTime_ms={}",
        buy_order_id, ask_price_str, buy_qty_str, buy_placed.transact_time
    );

    let ws_timeout = Duration::from_secs(90);
    let deadline = tokio::time::Instant::now() + ws_timeout;

    let (filled_base, buy_fill_ms) = loop {
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
        if update.side != MexcTradeSide::Buy {
            continue;
        }
        if !matches!(
            update.order_type,
            MexcOrderType::ImmediateOrCancel | MexcOrderType::Limit
        ) {
            continue;
        }

        let d = update.to_display();
        println!(
            "WS buy: status={} action={} executed={} | send_time_ms={} display_time_ms={}",
            d.status, d.event_action, d.executed, update.send_time, d.time
        );

        if matches!(update.status, MexcOrderStatus::Filled) {
            break (update.filled_quantity(), d.time);
        }
    };

    assert!(
        filled_base > 0.0,
        "IOC buy must fill; got filled_base={filled_base}"
    );

    let sell_qty_str = format_base_qty(filled_base);

    let book2 = mexc
        .get_price(&symbol)
        .await
        .expect("get_price before sell");
    let bid2 = book2.bid_price;
    let bid_price_str = format_price(bid2);
    println!(
        "Book before sell: bid={} → IOC SELL qty={} @ bid price={}",
        bid2, sell_qty_str, bid_price_str
    );

    let sell_placed = mexc
        .place_limit_order(
            &symbol,
            MexcTradeSide::Sell,
            &bid_price_str,
            &sell_qty_str,
            MexcLimitOrderType::ImmediateOrCancel,
        )
        .await
        .expect("IOC limit sell");

    let sell_order_id = sell_placed.order_id.clone();
    println!(
        "Placed IOC SELL order_id={} price={} qty={} REST_transactTime_ms={}",
        sell_order_id, bid_price_str, sell_qty_str, sell_placed.transact_time
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
        if update.side != MexcTradeSide::Sell {
            continue;
        }
        if !matches!(
            update.order_type,
            MexcOrderType::ImmediateOrCancel | MexcOrderType::Limit
        ) {
            continue;
        }

        let d = update.to_display();
        println!(
            "WS sell: status={} action={} executed={} | send_time_ms={} display_time_ms={}",
            d.status, d.event_action, d.executed, update.send_time, d.time
        );

        if matches!(update.status, MexcOrderStatus::Filled) {
            let sell_fill_ms = d.time;
            let delta_ws_ms = sell_fill_ms.saturating_sub(buy_fill_ms);
            println!(
                "IOC round-trip done. WS Δ buy→sell fill: {} ms ({:.3} s)",
                delta_ws_ms,
                delta_ws_ms as f64 / 1000.0
            );
            return;
        }
    }
}
