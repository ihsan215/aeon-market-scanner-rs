use aeon_market_scanner_rs::CexExchange;

// Allow dead code warnings since different test files use different items from this module
#[allow(dead_code)]
pub const QUOTE_AMOUNT: f64 = 1000.0;

pub const TEST_SYMBOL: &str = "BNBUSDT";

/// Helper function to get all CEX exchanges
#[allow(dead_code)]
pub fn get_all_cex_exchanges() -> Vec<CexExchange> {
    vec![
        CexExchange::Binance,
        CexExchange::Bybit,
        CexExchange::MEXC,
        CexExchange::OKX,
        CexExchange::Gateio,
        CexExchange::Kucoin,
        CexExchange::Bitget,
        CexExchange::Btcturk,
        CexExchange::Htx,
        CexExchange::Coinbase,
        CexExchange::Kraken,
        CexExchange::Bitfinex,
        CexExchange::Upbit,
        CexExchange::Cryptocom,
    ]
}
