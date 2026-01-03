// src/lib.rs
pub mod cex;
pub mod common;

// Re-export common types
pub use cex::{Binance, Mexc};
pub use common::{
    CexExchange, CexPrice, DexAggregator, Exchange, ExchangeTrait, MarketScannerError,
};
