pub mod client;
pub mod commission;
pub mod errors;
pub mod exchange;
pub mod price;
pub mod utils;

// Re-export
pub use client::create_http_client;
pub use commission::{AmountSide, effective_price, fee_rate, taker_fee_rate};
pub use errors::MarketScannerError;
pub use exchange::{CEXTrait, CexExchange, DEXTrait, DexAggregator, Exchange, ExchangeTrait};
pub use price::{CexPrice, DexPrice, DexRouteSummary};
pub use utils::{
    find_mid_price, format_symbol_for_exchange, format_symbol_for_exchange_ws,
    get_timestamp_millis, normalize_symbol, parse_f64, standard_symbol_for_cex_ws_response,
};
