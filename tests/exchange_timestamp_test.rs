//! `CexPrice::exchange_timestamp` (0.9.0): the venue's own event time on WebSocket prices.
//! Live API tests. Run: cargo test --test exchange_timestamp_test -- --nocapture

use aeon_market_scanner_rs::{
    Binance, Bitfinex, Bitget, Bybit, CEXTrait, CexPrice, Coinbase, Cryptocom, Gateio, Kraken,
    Kucoin, Mexc, OKX, Upbit,
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Allowed difference between the exchange clock and ours (clock skew + network delay).
const MAX_SKEW_MS: u64 = 60_000;

async fn first_price<E: CEXTrait>(ex: E, symbol: &str) -> CexPrice {
    let mut rx = ex
        .stream_price_websocket(&[symbol], 3, 2000)
        .await
        .expect("stream");
    tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out waiting for a price")
        .expect("stream closed")
}

fn assert_event_time(name: &str, p: &CexPrice) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let et = p
        .exchange_timestamp
        .unwrap_or_else(|| panic!("{name}: expected exchange_timestamp, got None"));
    let diff = now.abs_diff(et);
    println!(
        "{name:<10} exchange_timestamp={et} local={} lag={}ms",
        p.timestamp,
        p.timestamp as i64 - et as i64
    );
    assert!(
        diff < MAX_SKEW_MS,
        "{name}: exchange_timestamp {et} is {diff}ms away from now"
    );
    assert!(
        p.timestamp >= et.saturating_sub(MAX_SKEW_MS),
        "{name}: receive time before event time"
    );
}

#[tokio::test]
async fn bybit_has_event_time() {
    assert_event_time("Bybit", &first_price(Bybit::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn okx_has_event_time() {
    assert_event_time("OKX", &first_price(OKX::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn kucoin_has_event_time() {
    assert_event_time("KuCoin", &first_price(Kucoin::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn bitget_has_event_time() {
    assert_event_time("Bitget", &first_price(Bitget::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn coinbase_has_event_time() {
    assert_event_time("Coinbase", &first_price(Coinbase::new(), "BTCUSD").await);
}
#[tokio::test]
async fn kraken_has_event_time() {
    assert_event_time("Kraken", &first_price(Kraken::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn upbit_has_event_time() {
    assert_event_time("Upbit", &first_price(Upbit::new(), "BTCUSDT").await);
}
#[tokio::test]
async fn cryptocom_has_event_time() {
    assert_event_time(
        "Crypto.com",
        &first_price(Cryptocom::new(), "BTCUSDT").await,
    );
}
#[tokio::test]
async fn mexc_has_event_time() {
    assert_event_time("MEXC", &first_price(Mexc::new(), "BTCUSDT").await);
}

/// Feeds without an event time keep `None` (and still report local receive time).
#[tokio::test]
async fn feeds_without_event_time_report_none() {
    let b = first_price(Binance::new(), "BTCUSDT").await;
    let f = first_price(Bitfinex::new(), "BTCUSDT").await;
    let g = first_price(Gateio::new(), "BTCUSDT").await;
    for (name, p) in [("Binance", b), ("Bitfinex", f), ("Gate", g)] {
        assert!(p.exchange_timestamp.is_none(), "{name}: expected None");
        assert!(p.timestamp > 0, "{name}: local timestamp still set");
    }
}

/// REST prices carry no event time.
#[tokio::test]
async fn rest_prices_have_no_event_time() {
    let p = OKX::new().get_price("BTCUSDT").await.expect("rest price");
    assert!(p.exchange_timestamp.is_none());
}

#[test]
fn serialization_skips_none_and_keeps_some() {
    use aeon_market_scanner_rs::{CexExchange, Exchange};
    let mut p = CexPrice {
        symbol: "BTCUSDT".into(),
        mid_price: 1.0,
        bid_price: 1.0,
        ask_price: 1.0,
        bid_qty: 1.0,
        ask_qty: 1.0,
        timestamp: 2,
        exchange: Exchange::Cex(CexExchange::OKX),
        exchange_timestamp: None,
    };
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("exchange_timestamp"));
    // Old payloads without the field still deserialize.
    let back: CexPrice = serde_json::from_str(&json).unwrap();
    assert!(back.exchange_timestamp.is_none());
    p.exchange_timestamp = Some(1);
    assert!(
        serde_json::to_string(&p)
            .unwrap()
            .contains(r#""exchange_timestamp":1"#)
    );
}
