//! Stock prices and fiat rates through fallback chains.
//!
//! Run: `cargo run --example fallback`
//! (add `--features unofficial-yahoo` to put Yahoo in both chains)
//!
//! Keys come from `.env` / the environment: ALPACA_API_KEY_ID + ALPACA_API_SECRET_KEY,
//! TIINGO_API_TOKEN, FINNHUB_API_KEY. Fiat works without any key.

use aeon_market_scanner_rs::common::get_timestamp_millis;
use aeon_market_scanner_rs::{FiatFallback, FiatTrait, StockFallback, StockTrait};

/// "12s", "3h" ... since a provider timestamp (ms), or "?" when unknown.
fn age(exchange_timestamp: Option<u64>) -> String {
    let Some(t) = exchange_timestamp else {
        return "?".to_string();
    };
    let s = get_timestamp_millis().saturating_sub(t) / 1000;
    match s {
        0..=119 => format!("{s}s"),
        120..=7199 => format!("{}m", s / 60),
        _ => format!("{}h", s / 3600),
    }
}

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let _ = dotenvy::dotenv();

    // ---------- Stocks ----------
    match StockFallback::from_env() {
        Err(e) => println!("stocks skipped: {e}"),
        Ok(stocks) => {
            println!("stock chain: {:?}", stocks.providers().collect::<Vec<_>>());

            for p in stocks
                .get_prices(&["AAPL", "MSFT", "BRK.B", "NOTAREALTICKER"])
                .await?
            {
                println!(
                    "  {:<6} {:>10.2}  from {:<8?} age {:>4}  bid/ask {:?}/{:?}",
                    p.symbol,
                    p.price,
                    p.provider,
                    age(p.exchange_timestamp),
                    p.bid_price,
                    p.ask_price
                );
            }
        }
    }

    // ---------- Fiat ----------
    let fx = FiatFallback::from_env();
    println!("fiat chain: {:?}", fx.providers().collect::<Vec<_>>());

    for r in fx
        .get_rates(&["EUR/USD", "USD/TRY", "GBP/JPY", "USD/ARS"])
        .await?
    {
        println!(
            "  {:<8} {:>12.5}  from {:<15?} age {:>4}  bid/ask {:?}/{:?}",
            r.pair,
            r.rate,
            r.provider,
            age(r.exchange_timestamp),
            r.bid,
            r.ask
        );
    }

    Ok(())
}
