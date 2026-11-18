use aeon_market_scanner_rs::{FiatProvider, FiatTrait, MarketScannerError, Tcmb};

#[tokio::test]
async fn test_tcmb_health_check() {
    assert!(Tcmb::new().health_check().await.is_ok());
}

#[tokio::test]
async fn test_tcmb_rates_have_bid_ask() {
    let pairs = ["USD/TRY", "EUR/TRY", "TRY/USD", "EUR/USD", "JPY/TRY"];
    let rates = Tcmb::new().get_rates(&pairs).await.unwrap();
    let got: Vec<&str> = rates.iter().map(|r| r.pair.as_str()).collect();
    assert_eq!(got, pairs);
    for rate in &rates {
        let (bid, ask) = (rate.bid.unwrap(), rate.ask.unwrap());
        println!(
            "{} bid={bid} ask={ask} mid={} ({})",
            rate.pair, rate.rate, rate.date
        );
        assert!(bid > 0.0 && bid <= rate.rate && rate.rate <= ask);
        assert_eq!(rate.provider, FiatProvider::Tcmb);
    }
    // JPY is published per 100 units; the provider normalises to 1.
    let jpy = &rates[4];
    assert!(
        jpy.rate < 5.0,
        "JPY/TRY should be per 1 JPY, got {}",
        jpy.rate
    );
}

#[tokio::test]
async fn test_tcmb_get_all_rates() {
    let rates = Tcmb::new().get_all_rates().await.unwrap();
    assert!(rates.len() >= 15, "got {}", rates.len());
    assert!(rates.iter().all(|r| r.quote == "TRY"));
    assert!(rates.iter().any(|r| r.base == "USD"));
}

#[tokio::test]
async fn test_tcmb_unsupported_currency() {
    assert!(matches!(
        Tcmb::new().get_rate("BRL/TRY").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
