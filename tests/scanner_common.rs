use aeon_market_scanner_rs::CexExchange;
use aeon_market_scanner_rs::dex::chains::{ChainId, Token};

// Allow dead code warnings since different test files use different items from this module
#[allow(dead_code)]
pub const QUOTE_AMOUNT: f64 = 1000.0;

#[allow(dead_code)]
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

/// Helper functions for common tokens (optional convenience functions)
/// These use Token::create() from the token module
#[allow(dead_code)]
pub fn create_bsc_bnb() -> Token {
    Token::create(
        "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
        "BNB",
        "BNB",
        18,
        ChainId::BSC,
    )
}

#[allow(dead_code)]
pub fn create_bsc_usdt() -> Token {
    Token::create(
        "0x55d398326f99059fF775485246999027B3197955",
        "Tether USD",
        "USDT",
        18,
        ChainId::BSC,
    )
}

#[allow(dead_code)]
pub fn create_bsc_usdc() -> Token {
    Token::create(
        "0x8AC76a51cc950d9822D68b83fE1Ad97B32Cd580d",
        "USD Coin",
        "USDC",
        18,
        ChainId::BSC,
    )
}
#[allow(dead_code)]
pub fn create_eth_eth() -> Token {
    Token::create(
        "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
        "Ether",
        "ETH",
        18,
        ChainId::ETHEREUM,
    )
}

#[allow(dead_code)]
pub fn create_eth_usdt() -> Token {
    Token::create(
        "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        "Tether USD",
        "USDT",
        6,
        ChainId::ETHEREUM,
    )
}
#[allow(dead_code)]
pub fn create_eth_usdc() -> Token {
    Token::create(
        "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48",
        "USD Coin",
        "USDC",
        6,
        ChainId::ETHEREUM,
    )
}

#[allow(dead_code)]
pub fn create_base_eth() -> Token {
    Token::create(
        "0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE",
        "Ether",
        "ETH",
        18,
        ChainId::BASE,
    )
}

#[allow(dead_code)]
pub fn create_base_usdc() -> Token {
    Token::create(
        "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913",
        "USDC",
        "USDC",
        6,
        ChainId::BASE,
    )
}
