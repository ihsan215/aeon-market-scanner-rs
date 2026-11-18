//! Live tests for the unofficial Yahoo provider.
//! Run with: `cargo test --features unofficial-yahoo --test fiat_yahoo_test`
#![cfg(feature = "unofficial-yahoo")]

use aeon_market_scanner_rs::common::get_timestamp_millis;
use aeon_market_scanner_rs::{FiatProvider, FiatTrait, MarketScannerError, YahooFiat};

#[tokio::test]
async fn test_yahoo_health_check() {
    assert!(YahooFiat::new().health_check().await.is_ok());
}

#[tokio::test]
async fn test_yahoo_get_rates_keeps_order() {
    let pairs = ["EUR/USD", "USD/TRY", "GBP/JPY", "usd-brl"];
    let rates = YahooFiat::new().get_rates(&pairs).await.unwrap();
    let got: Vec<&str> = rates.iter().map(|r| r.pair.as_str()).collect();
    assert_eq!(got, ["EUR/USD", "USD/TRY", "GBP/JPY", "USD/BRL"]);
    for rate in &rates {
        let age_s = rate
            .exchange_timestamp
            .map(|t| get_timestamp_millis().saturating_sub(t) / 1000);
        println!("{} = {} (age {:?}s)", rate.pair, rate.rate, age_s);
        assert!(rate.rate > 0.0);
        assert!(rate.exchange_timestamp.is_some());
        assert_eq!(rate.provider, FiatProvider::Yahoo);
    }
}

#[tokio::test]
async fn test_yahoo_invalid_pair() {
    assert!(matches!(
        YahooFiat::new().get_rate("XXX/YYY").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
