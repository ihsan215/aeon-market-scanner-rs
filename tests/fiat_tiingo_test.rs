//! Live Tiingo FX tests. Skipped unless `TIINGO_API_TOKEN` is set (a `.env` file works).

use aeon_market_scanner_rs::common::get_timestamp_millis;
use aeon_market_scanner_rs::{FiatProvider, FiatTrait, MarketScannerError, TiingoFiat};

fn client() -> Option<TiingoFiat> {
    let _ = dotenvy::dotenv();
    TiingoFiat::from_env()
        .map_err(|_| println!("TIINGO_API_TOKEN not set, skipping"))
        .ok()
}

#[tokio::test]
async fn test_tiingo_fx_rates_have_bid_ask_and_keep_order() {
    let Some(fx) = client() else { return };
    fx.health_check().await.unwrap();

    // TRY/USD is only listed as usdtry, so it is fetched inverted.
    let pairs = ["EUR/USD", "usd-try", "GBP/JPY", "TRY/USD", "EUR/USD"];
    let rates = fx.get_rates(&pairs).await.unwrap();
    let got: Vec<&str> = rates.iter().map(|r| r.pair.as_str()).collect();
    assert_eq!(got, ["EUR/USD", "USD/TRY", "GBP/JPY", "TRY/USD", "EUR/USD"]);

    for r in &rates {
        let age_s = r
            .exchange_timestamp
            .map(|t| get_timestamp_millis().saturating_sub(t) / 1000);
        println!(
            "{} bid={:?} ask={:?} mid={} age={:?}s",
            r.pair, r.bid, r.ask, r.rate, age_s
        );
        assert_eq!(r.provider, FiatProvider::Tiingo);
        let (bid, ask) = (r.bid.unwrap(), r.ask.unwrap());
        assert!(bid > 0.0 && bid <= r.rate && r.rate <= ask);
        assert!(r.exchange_timestamp.is_some());
    }

    let (usd_try, try_usd) = (&rates[1], &rates[3]);
    assert!((usd_try.rate * try_usd.rate - 1.0).abs() < 1e-3);
}

#[tokio::test]
async fn test_tiingo_fx_unknown_pair() {
    let Some(fx) = client() else { return };
    assert!(matches!(
        fx.get_rate("XXX/YYY").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
