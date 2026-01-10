pub mod client;
pub mod errors;
pub mod exchange;
pub mod price;
pub mod utils;

// Re-export
pub use client::create_http_client;
pub use errors::MarketScannerError;
pub use exchange::{CEXTrait, CexExchange, DexAggregator, Exchange, ExchangeTrait};
pub use price::CexPrice;
pub use utils::{
    find_mid_price, format_symbol_for_exchange, get_timestamp_millis, normalize_symbol, parse_f64,
};
