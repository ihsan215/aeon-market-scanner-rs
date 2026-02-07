// src/common/utils.rs
use crate::common::{CexExchange, MarketScannerError};

// Parse a string to a f64, return a MarketScannerError if the parsing fails
pub fn parse_f64(value: &str, field_name: &str) -> Result<f64, MarketScannerError> {
    value
        .parse::<f64>()
        .map_err(|_| MarketScannerError::ApiError(format!("Invalid {} format", field_name)))
}

// Find mid price between bid and ask price
pub fn find_mid_price(bid_price: f64, ask_price: f64) -> f64 {
    (bid_price + ask_price) / 2.0
}

// get timestamp in milliseconds
pub fn get_timestamp_millis() -> u64 {
    chrono::Utc::now()
        .timestamp_millis()
        .try_into()
        .unwrap_or(0)
}

/// Normalize symbol to common format (uppercase, no separators)
/// Accepts formats like: BTCUSDT, BTC-USDT, BTC_USDT, btcusdt
pub fn normalize_symbol(symbol: &str) -> String {
    symbol.to_uppercase().replace('-', "").replace('_', "")
}

/// Convert common symbol format (e.g., BTCUSDT) to exchange-specific format
/// Common format: BTCUSDT (uppercase, no separators)
pub fn format_symbol_for_exchange(
    symbol: &str,
    exchange: &CexExchange,
) -> Result<String, MarketScannerError> {
    // First normalize the input symbol
    let normalized = normalize_symbol(symbol);

    // Validate normalized symbol is not empty
    if normalized.is_empty() {
        return Err(MarketScannerError::InvalidSymbol(
            "Symbol cannot be empty".to_string(),
        ));
    }

    // Convert to exchange-specific format
    let formatted = match exchange {
        // Exchanges using standard format: BTCUSDT (uppercase, no separators)
        CexExchange::Binance
        | CexExchange::Bybit
        | CexExchange::MEXC
        | CexExchange::Bitget
        | CexExchange::Btcturk => normalized,

        // Exchanges using dash separator: BTC-USDT
        CexExchange::OKX | CexExchange::Kucoin => {
            // Split at USDT (4 chars) or USD (3 chars) or other common quote currencies
            if normalized.len() >= 7 && normalized.ends_with("USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 && normalized.ends_with("USD") {
                let split_point = normalized.len() - 3;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 {
                // Generic split: assume last 3 chars are quote currency
                let split_point = normalized.len() - 3;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Symbol too short for {:?} format: {}",
                    exchange, normalized
                )));
            }
        }

        // Coinbase uses dash separator: BTC-USDT or BTC-USD
        CexExchange::Coinbase => {
            if normalized.len() >= 7 && normalized.ends_with("USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 && normalized.ends_with("USD") {
                let split_point = normalized.len() - 3;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 {
                let split_point = normalized.len() - 3;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Symbol too short for Coinbase format: {}",
                    normalized
                )));
            }
        }

        // HTX uses lowercase: btcusdt
        CexExchange::Htx => normalized.to_lowercase(),

        // Kraken uses XBT instead of BTC: XBTUSDT
        CexExchange::Kraken => {
            if normalized.starts_with("BTC") {
                normalized.replace("BTC", "XBT")
            } else {
                normalized
            }
        }

        // Gate.io uses underscore separator: BTC_USDT
        CexExchange::Gateio => {
            if normalized.len() >= 7 && normalized.ends_with("USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 && normalized.ends_with("USD") {
                let split_point = normalized.len() - 3;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 {
                let split_point = normalized.len() - 3;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Symbol too short for Gate.io format: {}",
                    normalized
                )));
            }
        }

        // Bitfinex uses prefix "t": tBTCUSD or tBTCUST
        // Note: Bitfinex uses BTCUST instead of BTCUSDT
        CexExchange::Bitfinex => {
            // Bitfinex requires "t" prefix for trading pairs
            // Convert USDT to UST for Bitfinex
            let bitfinex_symbol = if normalized.ends_with("USDT") {
                normalized.replace("USDT", "UST")
            } else {
                normalized
            };
            format!("t{}", bitfinex_symbol)
        }

        // Upbit uses format: KRW-BTC, USDT-BTC, BTC-ETH (dash separator, quote-base)
        CexExchange::Upbit => {
            // Upbit uses quote-base format with dash: KRW-BTC, USDT-BTC
            // For BTCUSDT, we convert to USDT-BTC (quote-base)
            // For BTCUSD, we convert to KRW-BTC (if USD, use KRW as default)
            if normalized.len() >= 7 && normalized.ends_with("USDT") {
                // BTCUSDT -> USDT-BTC
                let split_point = normalized.len() - 4;
                format!("USDT-{}", &normalized[..split_point])
            } else if normalized.len() >= 6 && normalized.ends_with("KRW") {
                // BTCKRW -> KRW-BTC
                let split_point = normalized.len() - 3;
                format!("KRW-{}", &normalized[..split_point])
            } else if normalized.len() >= 6 && normalized.ends_with("USD") {
                // BTCUSD -> KRW-BTC (Upbit uses KRW instead of USD)
                let split_point = normalized.len() - 3;
                format!("KRW-{}", &normalized[..split_point])
            } else if normalized.len() >= 6 && normalized.ends_with("BTC") {
                // ETHBTC -> BTC-ETH
                let split_point = normalized.len() - 3;
                format!("BTC-{}", &normalized[..split_point])
            } else if normalized.starts_with("BTC") && normalized.len() >= 7 {
                // BTCETH -> BTC-ETH (base-quote stays same)
                let split_point = 3;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if normalized.len() >= 6 {
                // Generic: assume last 3-4 chars are quote
                let split_point = if normalized.len() >= 7 {
                    normalized.len() - 4
                } else {
                    normalized.len() - 3
                };
                format!(
                    "{}-{}",
                    &normalized[split_point..],
                    &normalized[..split_point]
                )
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Symbol too short for Upbit format: {}",
                    normalized
                )));
            }
        }

        // Crypto.com Exchange uses format: BTC_USDT (underscore separator)
        CexExchange::Cryptocom => {
            // Crypto.com Exchange uses underscore separator: BTC_USDT
            if normalized.len() >= 7 && normalized.ends_with("USDT") {
                let split_point = normalized.len() - 4;
                format!("{}_{}", &normalized[..split_point], &normalized[split_point..])
            } else if normalized.len() >= 6 && normalized.ends_with("USD") {
                let split_point = normalized.len() - 3;
                format!("{}_{}", &normalized[..split_point], &normalized[split_point..])
            } else if normalized.len() >= 6 && normalized.ends_with("BTC") {
                let split_point = normalized.len() - 3;
                format!("{}_{}", &normalized[..split_point], &normalized[split_point..])
            } else if normalized.len() >= 6 {
                let split_point = normalized.len() - 3;
                format!("{}_{}", &normalized[..split_point], &normalized[split_point..])
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Symbol too short for Crypto.com format: {}",
                    normalized
                )));
            }
        }
    };

    Ok(formatted)
}

/// Symbol string to use for WebSocket subscribe/connect for the given exchange.
/// Same as [format_symbol_for_exchange] for most exchanges; Binance uses lowercase for stream name.
pub fn format_symbol_for_exchange_ws(
    symbol: &str,
    exchange: &CexExchange,
) -> Result<String, MarketScannerError> {
    let formatted = format_symbol_for_exchange(symbol, exchange)?;
    let ws_symbol = match exchange {
        CexExchange::Binance => formatted.to_lowercase(),
        CexExchange::Kraken => {
            // WS v2 uses BASE/QUOTE format (e.g. BTC/USDT) - readable, not XBT
            let n = crate::common::normalize_symbol(symbol);
            if n.len() >= 7 && n.ends_with("USDT") {
                format!("{}/USDT", &n[..n.len() - 4])
            } else if n.len() >= 6 && n.ends_with("USD") {
                format!("{}/USD", &n[..n.len() - 3])
            } else if n.len() >= 6 {
                let split = n.len() - 3;
                format!("{}/{}", &n[..split], &n[split..])
            } else {
                formatted
            }
        }
        _ => formatted,
    };
    Ok(ws_symbol)
}

/// Standard symbol string for [CexPrice] when returning from WebSocket (same format as REST).
/// E.g. Bitfinex uses UST instead of USDT in the pair name.
pub fn standard_symbol_for_cex_ws_response(symbol: &str, exchange: &CexExchange) -> String {
    let normalized = normalize_symbol(symbol);
    match exchange {
        CexExchange::Bitfinex if normalized.ends_with("USDT") => {
            normalized.replace("USDT", "UST")
        }
        _ => normalized,
    }
}
