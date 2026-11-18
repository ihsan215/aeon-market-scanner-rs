//! Live Finnhub and Tiingo tests. Each is skipped unless its key is set
//! (`FINNHUB_API_KEY`, `TIINGO_API_TOKEN`; a `.env` file works).

use aeon_market_scanner_rs::{Finnhub, MarketScannerError, StockProvider, StockTrait, Tiingo};

fn finnhub() -> Option<Finnhub> {
    let _ = dotenvy::dotenv();
    Finnhub::from_env()
        .map_err(|_| println!("FINNHUB_API_KEY not set, skipping"))
        .ok()
}

fn tiingo() -> Option<Tiingo> {
    let _ = dotenvy::dotenv();
    Tiingo::from_env()
        .map_err(|_| println!("TIINGO_API_TOKEN not set, skipping"))
        .ok()
}

fn check(prices: &[aeon_market_scanner_rs::StockPrice], provider: StockProvider) {
    for p in prices {
        println!(
            "{} price={} last={:?} bid={:?} ask={:?} t={:?}",
            p.symbol, p.price, p.last_price, p.bid_price, p.ask_price, p.exchange_timestamp
        );
        assert!(p.price > 0.0);
        assert_eq!(p.provider, provider);
    }
}

#[tokio::test]
async fn test_finnhub_get_prices() {
    let Some(finnhub) = finnhub() else { return };
    finnhub.health_check().await.unwrap();
    let prices = finnhub
        .get_prices(&["AAPL", "MSFT", "brk-b", "NOTAREALTICKER"])
        .await
        .unwrap();
    check(&prices, StockProvider::Finnhub);
    assert!(prices.iter().any(|p| p.symbol == "AAPL"));
    assert!(!prices.iter().any(|p| p.symbol == "NOTAREALTICKER"));
}

#[tokio::test]
async fn test_finnhub_invalid_symbol() {
    let Some(finnhub) = finnhub() else { return };
    assert!(matches!(
        finnhub.get_price("NOTAREALTICKER").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}

#[tokio::test]
async fn test_tiingo_get_prices() {
    let Some(tiingo) = tiingo() else { return };
    tiingo.health_check().await.unwrap();
    let prices = tiingo.get_prices(&["AAPL", "MSFT", "brk-b"]).await.unwrap();
    check(&prices, StockProvider::Tiingo);
    assert!(prices.iter().any(|p| p.symbol == "BRK.B"));
}

#[tokio::test]
async fn test_tiingo_invalid_symbol() {
    let Some(tiingo) = tiingo() else { return };
    assert!(tiingo.get_price("NOTAREALTICKER").await.is_err());
}
