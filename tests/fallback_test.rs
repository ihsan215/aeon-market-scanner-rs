//! Live tests for StockFallback / FiatFallback built from whatever keys are in `.env`.

use std::collections::BTreeMap;

use aeon_market_scanner_rs::{
    FiatFallback, FiatTrait, MarketScannerError, SP500_SYMBOLS, StockFallback, StockTrait,
};

#[tokio::test]
async fn test_stock_fallback_from_env() {
    let _ = dotenvy::dotenv();
    let Ok(stocks) = StockFallback::from_env() else {
        println!("no stock provider keys set, skipping");
        return;
    };
    println!("stock chain: {:?}", stocks.providers().collect::<Vec<_>>());

    let aapl = stocks.get_price("AAPL").await.unwrap();
    println!(
        "AAPL {} from {:?} (t={:?})",
        aapl.price, aapl.provider, aapl.exchange_timestamp
    );
    assert!(aapl.price > 0.0);

    let few = stocks
        .get_prices(&["AAPL", "MSFT", "brk-b", "NOTAREALTICKER"])
        .await
        .unwrap();
    for p in &few {
        println!("{} {} from {:?}", p.symbol, p.price, p.provider);
    }
    assert_eq!(few.len(), 3);

    // The full index only goes to the primary provider (no age limit), so it does not spend
    // the later providers' quotas (Tiingo Starter: 500 symbols per month).
    let stocks = StockFallback::from_env().unwrap().with_max_age(None);
    let prices = stocks.get_sp500_prices().await.unwrap();
    let mut by_provider = BTreeMap::new();
    for p in &prices {
        *by_provider.entry(format!("{:?}", p.provider)).or_insert(0) += 1;
    }
    println!(
        "S&P 500: {} / {} prices, by provider {by_provider:?}",
        prices.len(),
        SP500_SYMBOLS.len()
    );
    assert!(prices.len() >= SP500_SYMBOLS.len() - 5);

    assert!(matches!(
        stocks.get_price("NOTAREALTICKER").await,
        Err(MarketScannerError::InvalidSymbol(_))
    ));
}

#[tokio::test]
async fn test_fiat_fallback_from_env() {
    let _ = dotenvy::dotenv();
    let fx = FiatFallback::from_env();
    println!("fiat chain: {:?}", fx.providers().collect::<Vec<_>>());

    // USD/ARS is not on Tiingo FX, so it has to come from a later provider.
    let pairs = ["EUR/USD", "USD/TRY", "GBP/JPY", "USD/ARS", "TRY/EUR"];
    let rates = fx.get_rates(&pairs).await.unwrap();
    assert_eq!(rates.len(), pairs.len());
    for r in &rates {
        println!(
            "{} = {} bid={:?} ask={:?} from {:?} (t={:?})",
            r.pair, r.rate, r.bid, r.ask, r.provider, r.exchange_timestamp
        );
        assert!(r.rate > 0.0);
    }

    assert!(fx.get_rate("XXX/YYY").await.is_err());
}
