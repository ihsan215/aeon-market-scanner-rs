use aeon_market_scanner_rs::{ExchangeRateApi, FiatProvider, FiatTrait, MarketScannerError};

#[tokio::test]
async fn test_er_api_health_check() {
    assert!(ExchangeRateApi::new().health_check().await.is_ok());
}

#[tokio::test]
async fn test_er_api_get_rates_keeps_order() {
    let pairs = ["EUR/USD", "USD/TRY", "GBP/USD", "USD/ARS", "EUR/TRY"];
    let rates = ExchangeRateApi::new().get_rates(&pairs).await.unwrap();
    let got: Vec<&str> = rates.iter().map(|r| r.pair.as_str()).collect();
    assert_eq!(got, pairs);
    for rate in &rates {
        println!("{} = {} ({})", rate.pair, rate.rate, rate.date);
        assert!(rate.rate > 0.0);
        assert_eq!(rate.date.len(), 10);
        assert!(rate.exchange_timestamp.is_some());
        assert_eq!(rate.provider, FiatProvider::ExchangeRateApi);
    }
}

#[tokio::test]
async fn test_er_api_get_all_rates() {
    let rates = ExchangeRateApi::new().get_all_rates("USD").await.unwrap();
    println!("{} currencies", rates.len());
    assert!(
        rates.len() >= 150,
        "expected 160+ currencies, got {}",
        rates.len()
    );
    assert!(!rates.iter().any(|r| r.quote == "USD"));
}

#[tokio::test]
async fn test_er_api_invalid_currency() {
    assert!(matches!(
        ExchangeRateApi::new().get_rate("XXX/USD").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
