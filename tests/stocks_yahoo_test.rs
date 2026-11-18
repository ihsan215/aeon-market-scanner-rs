//! Live tests for the unofficial Yahoo stock provider.
//! Run with: `cargo test --features unofficial-yahoo --test stocks_yahoo_test`
#![cfg(feature = "unofficial-yahoo")]

use aeon_market_scanner_rs::{MarketScannerError, StockProvider, StockTrait, YahooStocks};

#[tokio::test]
async fn test_yahoo_stocks_get_prices() {
    let yahoo = YahooStocks::new();
    yahoo.health_check().await.unwrap();
    let prices = yahoo
        .get_prices(&["AAPL", "brk-b", "SPY", "NOTAREALTICKER"])
        .await
        .unwrap();
    for p in &prices {
        println!(
            "{} price={} t={:?}",
            p.symbol, p.price, p.exchange_timestamp
        );
        assert!(p.price > 0.0);
        assert_eq!(p.last_price, Some(p.price));
        assert_eq!(p.provider, StockProvider::Yahoo);
    }
    let symbols: Vec<&str> = prices.iter().map(|p| p.symbol.as_str()).collect();
    assert_eq!(symbols, ["AAPL", "BRK.B", "SPY"]);
}

#[tokio::test]
async fn test_yahoo_stocks_invalid_symbol() {
    assert!(matches!(
        YahooStocks::new().get_price("NOTAREALTICKER").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}
