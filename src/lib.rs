pub mod cex;
pub mod common;
pub mod dex;

// Re-export common types
pub use cex::{
    Binance, Bitfinex, Bitget, Btcturk, Bybit, Coinbase, Cryptocom, Gateio, Htx, Kraken, Kucoin,
    Mexc, OKX, Upbit,
};

pub use common::{
    CEXTrait, CexExchange, CexPrice, DEXTrait, DexAggregator, Exchange, ExchangeTrait,
    MarketScannerError,
};
pub use dex::KyberSwap;
