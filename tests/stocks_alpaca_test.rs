//! Live Alpaca tests. They are skipped unless `ALPACA_API_KEY_ID` and
//! `ALPACA_API_SECRET_KEY` are set (a `.env` file works).
//!
//! Outside US market hours IEX often has no active bid/ask, so price assertions are
//! limited to quotes that came back.

use aeon_market_scanner_rs::{
    Alpaca, MarketScannerError, SP500_SYMBOLS, StockProvider, StockTrait,
};

fn client() -> Option<Alpaca> {
    let _ = dotenvy::dotenv();
    match Alpaca::from_env() {
        Ok(client) => Some(client),
        Err(_) => {
            println!("ALPACA_API_KEY_ID / ALPACA_API_SECRET_KEY not set, skipping");
            None
        }
    }
}

#[tokio::test]
async fn test_alpaca_health_check() {
    let Some(alpaca) = client() else { return };
    alpaca.health_check().await.unwrap();
}

#[tokio::test]
async fn test_alpaca_get_prices() {
    let Some(alpaca) = client() else { return };
    let prices = alpaca
        .get_prices(&["AAPL", "MSFT", "brk-b", "NVDA"])
        .await
        .unwrap();
    println!("got {} quotes", prices.len());
    for p in &prices {
        println!(
            "{} price={} bid={:?} ask={:?} t={:?}",
            p.symbol, p.price, p.bid_price, p.ask_price, p.exchange_timestamp
        );
        assert!(p.price > 0.0);
        if let (Some(bid), Some(ask)) = (p.bid_price, p.ask_price) {
            assert!(bid > 0.0 && bid <= p.price && p.price <= ask);
            assert!(
                (ask - bid) / p.price <= 0.02,
                "quote wider than the 2% default"
            );
        }
        assert_eq!(p.provider, StockProvider::Alpaca);
        assert!(["AAPL", "MSFT", "BRK.B", "NVDA"].contains(&p.symbol.as_str()));
    }
}

#[tokio::test]
async fn test_alpaca_get_sp500_prices() {
    let Some(alpaca) = client() else { return };
    let prices = alpaca.get_sp500_prices().await.unwrap();
    let with_quote = prices.iter().filter(|p| p.bid_price.is_some()).count();
    println!(
        "{} / {} S&P 500 prices ({} from a bid/ask quote)",
        prices.len(),
        SP500_SYMBOLS.len(),
        with_quote
    );
    // Every constituent should have at least a last trade or daily close, market open or not.
    assert!(
        prices.len() >= SP500_SYMBOLS.len() - 5,
        "only {} prices",
        prices.len()
    );
}

#[tokio::test]
async fn test_alpaca_invalid_symbol() {
    let Some(alpaca) = client() else { return };
    assert!(matches!(
        alpaca.get_price("").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
    assert!(alpaca.get_price("NOTAREALTICKER").await.is_err());
}
