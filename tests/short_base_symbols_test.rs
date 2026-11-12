//! Regression (0.8.0): pairs with 1-2 letter bases (OP, AR, ZK, S, ...) were formatted wrongly
//! on venues that use a separator (OKX/KuCoin/Coinbase: `OPU-SDT`, Gate/Crypto.com: `OPU_SDT`),
//! so these pairs could not be fetched or streamed. Live API tests.
//! Run: cargo test --test short_base_symbols_test -- --nocapture

mod common;

use aeon_market_scanner_rs::{CEXTrait, CexExchange, Exchange, Gateio, Kucoin, OKX};
use common::test_get_price_common;

#[tokio::test]
async fn okx_get_price_short_base() {
    test_get_price_common(
        &OKX::new(),
        "OPUSDT",
        Exchange::Cex(CexExchange::OKX),
        "OKX",
    )
    .await;
}

#[tokio::test]
async fn kucoin_get_price_short_base() {
    test_get_price_common(
        &Kucoin::new(),
        "OPUSDT",
        Exchange::Cex(CexExchange::Kucoin),
        "KuCoin",
    )
    .await;
}

#[tokio::test]
async fn gateio_get_price_short_base() {
    test_get_price_common(
        &Gateio::new(),
        "OPUSDT",
        Exchange::Cex(CexExchange::Gateio),
        "Gate.io",
    )
    .await;
}

#[tokio::test]
async fn okx_ws_short_bases() {
    let expected = ["OPUSDT", "ARUSDT"];
    let mut rx = OKX::new()
        .stream_price_websocket(&expected, 5, 5000)
        .await
        .expect("OKX WebSocket stream");

    let mut seen = std::collections::HashSet::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    while seen.len() < expected.len() {
        let p = tokio::time::timeout_at(deadline, rx.recv())
            .await
            .expect("timed out waiting for OPUSDT and ARUSDT")
            .expect("stream closed");
        assert!(p.bid_price > 0.0 && p.ask_price >= p.bid_price);
        assert!(
            expected.contains(&p.symbol.as_str()),
            "unexpected symbol {}",
            p.symbol
        );
        seen.insert(p.symbol);
    }
}
