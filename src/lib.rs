//! `aeon-market-scanner-rs`
//!
//! Fetch spot prices from multiple CEX/DEX venues and scan for arbitrage opportunities.
//!
//! ## Quickstart (REST)
//!
//! ```no_run
//! use aeon_market_scanner_rs::{Binance, CEXTrait};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
//! let price = Binance::new().get_price("BTCUSDT").await?;
//! println!("{} bid={} ask={}", price.symbol, price.bid_price, price.ask_price);
//! # Ok(())
//! # }
//! ```
//!
//! ## Quickstart (WebSocket stream)
//!
//! ```no_run
//! use aeon_market_scanner_rs::{Binance, CEXTrait};
//!
//! # #[tokio::main]
//! # async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
//! let mut rx = Binance::new()
//!     .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 10, 5000)
//!     .await?;
//!
//! while let Some(update) = rx.recv().await {
//!     println!("[{:?}] {} bid={} ask={}", update.exchange, update.symbol, update.bid_price, update.ask_price);
//! }
//! # Ok(())
//! # }
//! ```

pub mod cex;
pub mod common;
pub mod dex;
pub mod scanner;

// Re-export common types
pub use cex::{
    Binance, Bitfinex, Bitget, Btcturk, Bybit, Coinbase, Cryptocom, Gateio, Htx, Kraken, Kucoin,
    Mexc, OKX, Upbit,
};

pub use common::{
    AmountSide, CEXTrait, CexExchange, CexPrice, DEXTrait, DexAggregator, DexPrice,
    DexRouteSummary, Exchange, ExchangeTrait, FeeOverrides, MarketScannerError, effective_price,
    effective_price_with_overrides, fee_rate, fee_rate_with_overrides, taker_fee_rate,
    taker_fee_rate_with_overrides,
};
pub use dex::{
    KyberSwap, ListenMode, PoolKind, PriceDirection, PoolListenerConfig, PoolPriceUpdate,
    load_dotenv, stream_pool_prices,
};
pub use scanner::{ArbitrageOpportunity, ArbitrageScanner, PriceData};
