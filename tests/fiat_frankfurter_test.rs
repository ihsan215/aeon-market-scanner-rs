use aeon_market_scanner_rs::{FiatProvider, FiatTrait, Frankfurter, MarketScannerError};

#[tokio::test]
async fn test_frankfurter_health_check() {
    assert!(Frankfurter::new().health_check().await.is_ok());
}

#[tokio::test]
async fn test_frankfurter_get_rate() {
    let rate = Frankfurter::new().get_rate("eur-usd").await.unwrap();
    println!("{rate:?}");
    assert_eq!(rate.pair, "EUR/USD");
    assert_eq!(rate.base, "EUR");
    assert_eq!(rate.quote, "USD");
    assert!(rate.rate > 0.5 && rate.rate < 2.0, "EUR/USD out of range");
    assert_eq!(rate.date.len(), 10);
    assert_eq!(rate.provider, FiatProvider::Frankfurter);
}

#[tokio::test]
async fn test_frankfurter_get_rates_keeps_order() {
    let pairs = ["EUR/USD", "USD/TRY", "GBP/USD", "USD/JPY", "EUR/TRY"];
    let rates = Frankfurter::new().get_rates(&pairs).await.unwrap();
    let got: Vec<&str> = rates.iter().map(|r| r.pair.as_str()).collect();
    assert_eq!(got, pairs);
    for rate in &rates {
        println!("{} = {} ({})", rate.pair, rate.rate, rate.date);
        assert!(rate.rate > 0.0);
    }
}

#[tokio::test]
async fn test_frankfurter_get_all_rates() {
    let rates = Frankfurter::new().get_all_rates("USD").await.unwrap();
    assert!(
        rates.len() >= 20,
        "expected ~30 currencies, got {}",
        rates.len()
    );
    assert!(rates.iter().any(|r| r.pair == "USD/EUR"));
}

#[tokio::test]
async fn test_frankfurter_supported_currencies() {
    let currencies = Frankfurter::new().supported_currencies().await.unwrap();
    assert!(currencies.contains_key("USD"));
    assert!(currencies.contains_key("TRY"));
}

#[tokio::test]
async fn test_frankfurter_invalid_pairs() {
    let fx = Frankfurter::new();
    for pair in ["", "EURUSDT", "USD/USD"] {
        assert!(matches!(
            fx.get_rate(pair).await,
            Err(MarketScannerError::InvalidSymbol(_))
        ));
    }
    // Well-formed but unknown currency code.
    assert!(matches!(
        fx.get_rate("XXX/YYY").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
