//! Live CoinMarketCap tests. They use `CMC_API_KEY` when set, otherwise the keyless API
//! (shared IP rate pool), so they run without any key.

use aeon_market_scanner_rs::common::get_timestamp_millis;
use aeon_market_scanner_rs::{
    CoinMarketCap, MarketDataProvider, MarketDataTrait, MarketScannerError,
};

/// Unwraps, but treats the keyless quota running out (HTTP 429) as a skip, not a failure.
fn ok_or_skip<T>(cmc: &CoinMarketCap, result: Result<T, MarketScannerError>) -> Option<T> {
    match result {
        Err(MarketScannerError::ApiError(e)) if !cmc.has_api_key() && e.contains("429") => {
            println!("keyless CoinMarketCap quota used up, skipping: {e}");
            None
        }
        other => Some(other.unwrap()),
    }
}

fn client() -> CoinMarketCap {
    let _ = dotenvy::dotenv();
    CoinMarketCap::from_env()
}

#[tokio::test]
async fn test_cmc_prices_pick_the_real_assets() {
    let cmc = client();
    println!("keyed: {}", cmc.has_api_key());
    let Some(prices) = ok_or_skip(
        &cmc,
        cmc.get_prices(&["btc", "ETH", "SOL", "BTC", "NOTAREALCOIN123", "bad/sym"])
            .await,
    ) else {
        return;
    };
    let symbols: Vec<&str> = prices.iter().map(|p| p.symbol.as_str()).collect();
    assert_eq!(symbols, ["BTC", "ETH", "SOL"]);

    for p in &prices {
        let age = p
            .exchange_timestamp
            .map(|t| get_timestamp_millis().saturating_sub(t) / 1000);
        println!(
            "{} ({}, id {}) {} {} mcap={:?} 24h={:?}% rank={:?} age={:?}s",
            p.symbol,
            p.name,
            p.provider_id,
            p.price,
            p.quote,
            p.market_cap,
            p.percent_change_24h,
            p.rank,
            age
        );
        assert_eq!(p.provider, MarketDataProvider::CoinMarketCap);
        assert_eq!(p.quote, "USD");
        assert!(p.price > 0.0);
        assert!(
            p.rank.is_some_and(|r| r <= 50),
            "the well-known asset, not a look-alike"
        );
    }
    assert_eq!(prices[0].provider_id, "1");
    assert_eq!(prices[1].provider_id, "1027");
}

#[tokio::test]
async fn test_cmc_by_ids_and_convert() {
    let cmc = client().with_convert("EUR");
    let Some(prices) = ok_or_skip(&cmc, cmc.get_prices_by_ids(&[1027, 1, 999_999_999]).await)
    else {
        return;
    };
    let ids: Vec<&str> = prices.iter().map(|p| p.provider_id.as_str()).collect();
    assert_eq!(ids, ["1027", "1"]);
    assert!(prices.iter().all(|p| p.quote == "EUR" && p.price > 0.0));
}

#[tokio::test]
async fn test_cmc_unknown_symbol() {
    assert!(client().get_price("NOTAREALCOIN123").await.is_err());
}
