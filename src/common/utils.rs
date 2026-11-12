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

/// Epoch timestamp from a JSON number or numeric string, normalised to milliseconds
/// (seconds, micro- and nanoseconds are detected by magnitude). `None` if missing or implausible.
pub fn json_ms(value: Option<&serde_json::Value>) -> Option<u64> {
    let raw: u64 = match value? {
        serde_json::Value::Number(n) => n.as_u64().or_else(|| n.as_f64().map(|f| f as u64))?,
        serde_json::Value::String(s) => s.trim().parse().ok()?,
        _ => return None,
    };
    epoch_to_ms(raw)
}

fn epoch_to_ms(raw: u64) -> Option<u64> {
    let ms = match raw {
        0 => return None,
        r if r < 100_000_000_000 => r * 1_000, // seconds
        r if r < 100_000_000_000_000 => r,     // milliseconds
        r if r < 100_000_000_000_000_000 => r / 1_000, // microseconds
        r => r / 1_000_000,                    // nanoseconds
    };
    Some(ms)
}

/// RFC 3339 / ISO 8601 time (e.g. `2026-11-12T08:26:41.099248Z`) to epoch milliseconds.
pub fn iso8601_ms(s: &str) -> Option<u64> {
    chrono::DateTime::parse_from_rfc3339(s.trim())
        .ok()
        .and_then(|d| u64::try_from(d.timestamp_millis()).ok())
}

/// True if `normalized` ends with `quote` and has a non-empty base (e.g. `SUSDT` -> base `S`).
fn has_quote(normalized: &str, quote: &str) -> bool {
    normalized.len() > quote.len() && normalized.ends_with(quote)
}

/// Bitfinex pair name (`tBTCUST`, `tDOGE:UST`, `BTCUST`) -> standard symbol (`BTCUSDT`).
/// Bitfinex calls USDT "UST"; we always report the standard `USDT` quote.
pub fn bitfinex_symbol_to_standard(symbol: &str) -> String {
    let trimmed = symbol.trim();
    // Trading pairs are prefixed with a lowercase `t` (funding currencies with `f`).
    let without_prefix = trimmed.strip_prefix('t').unwrap_or(trimmed);
    let normalized = normalize_symbol(&without_prefix.replace(':', ""));
    match normalized.strip_suffix("UST") {
        Some(base) if !base.is_empty() => format!("{base}USDT"),
        _ => normalized,
    }
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
            if has_quote(&normalized, "USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if has_quote(&normalized, "USD") {
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
            if has_quote(&normalized, "USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}-{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if has_quote(&normalized, "USD") {
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
            if has_quote(&normalized, "USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if has_quote(&normalized, "USD") {
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
            let bitfinex_symbol = match normalized.strip_suffix("USDT") {
                Some(base) => format!("{base}UST"),
                None => normalized,
            };
            format!("t{}", bitfinex_symbol)
        }

        // Upbit uses format: KRW-BTC, USDT-BTC, BTC-ETH (dash separator, quote-base)
        CexExchange::Upbit => {
            // Upbit uses quote-base format with dash: KRW-BTC, USDT-BTC
            // For BTCUSDT, we convert to USDT-BTC (quote-base)
            // For BTCUSD, we convert to KRW-BTC (if USD, use KRW as default)
            if has_quote(&normalized, "USDT") {
                // BTCUSDT -> USDT-BTC
                let split_point = normalized.len() - 4;
                format!("USDT-{}", &normalized[..split_point])
            } else if has_quote(&normalized, "KRW") {
                // BTCKRW -> KRW-BTC
                let split_point = normalized.len() - 3;
                format!("KRW-{}", &normalized[..split_point])
            } else if has_quote(&normalized, "USD") {
                // BTCUSD -> KRW-BTC (Upbit uses KRW instead of USD)
                let split_point = normalized.len() - 3;
                format!("KRW-{}", &normalized[..split_point])
            } else if has_quote(&normalized, "BTC") {
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
            if has_quote(&normalized, "USDT") {
                let split_point = normalized.len() - 4;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if has_quote(&normalized, "USD") {
                let split_point = normalized.len() - 3;
                format!(
                    "{}_{}",
                    &normalized[..split_point],
                    &normalized[split_point..]
                )
            } else if has_quote(&normalized, "BTC") {
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
            if has_quote(&n, "USDT") {
                format!("{}/USDT", &n[..n.len() - 4])
            } else if has_quote(&n, "USD") {
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
/// E.g. Bitfinex `tBTCUST` / `tDOGE:UST` -> `BTCUSDT` / `DOGEUSDT`.
pub fn standard_symbol_for_cex_ws_response(symbol: &str, exchange: &CexExchange) -> String {
    let normalized = normalize_symbol(symbol);
    match exchange {
        CexExchange::Bitfinex => bitfinex_symbol_to_standard(symbol),
        // Upbit uses quote-base: USDT-BTC -> BTCUSDT, KRW-BTC -> BTCKRW
        CexExchange::Upbit if symbol.contains('-') => {
            let parts: Vec<&str> = symbol.split('-').collect();
            if parts.len() == 2 {
                format!("{}{}", parts[1].trim(), parts[0].trim()).to_uppercase()
            } else {
                normalized
            }
        }
        _ => normalized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(symbol: &str, ex: CexExchange) -> String {
        format_symbol_for_exchange(symbol, &ex).unwrap()
    }

    #[test]
    fn short_bases_split_correctly_on_dash_and_underscore_venues() {
        // 1-2 letter bases used to fall through to the generic "last 3 chars" split
        // (OPUSDT -> OPU-SDT) or error out (SUSDT).
        for (sym, dash, underscore) in [
            ("OPUSDT", "OP-USDT", "OP_USDT"),
            ("ARUSDT", "AR-USDT", "AR_USDT"),
            ("ZKUSDT", "ZK-USDT", "ZK_USDT"),
            ("SUSDT", "S-USDT", "S_USDT"),
            ("BTCUSDT", "BTC-USDT", "BTC_USDT"),
            ("1INCHUSDT", "1INCH-USDT", "1INCH_USDT"),
        ] {
            assert_eq!(fmt(sym, CexExchange::OKX), dash, "OKX {sym}");
            assert_eq!(fmt(sym, CexExchange::Kucoin), dash, "KuCoin {sym}");
            assert_eq!(fmt(sym, CexExchange::Coinbase), dash, "Coinbase {sym}");
            assert_eq!(fmt(sym, CexExchange::Gateio), underscore, "Gate {sym}");
            assert_eq!(
                fmt(sym, CexExchange::Cryptocom),
                underscore,
                "Crypto.com {sym}"
            );
        }
    }

    #[test]
    fn short_bases_with_usd_quote() {
        assert_eq!(fmt("OPUSD", CexExchange::OKX), "OP-USD");
        assert_eq!(fmt("SUSD", CexExchange::Coinbase), "S-USD");
        assert_eq!(fmt("ZKUSD", CexExchange::Cryptocom), "ZK_USD");
    }

    #[test]
    fn upbit_short_bases() {
        assert_eq!(fmt("OPUSDT", CexExchange::Upbit), "USDT-OP");
        assert_eq!(fmt("SUSDT", CexExchange::Upbit), "USDT-S");
        assert_eq!(fmt("OPKRW", CexExchange::Upbit), "KRW-OP");
        assert_eq!(fmt("OPBTC", CexExchange::Upbit), "BTC-OP");
        assert_eq!(fmt("BTCUSDT", CexExchange::Upbit), "USDT-BTC");
        assert_eq!(fmt("ETHBTC", CexExchange::Upbit), "BTC-ETH");
    }

    #[test]
    fn kraken_ws_short_bases() {
        let ws = |s| format_symbol_for_exchange_ws(s, &CexExchange::Kraken).unwrap();
        assert_eq!(ws("OPUSDT"), "OP/USDT");
        assert_eq!(ws("SUSDT"), "S/USDT");
        assert_eq!(ws("BTCUSD"), "BTC/USD");
    }

    #[test]
    fn existing_formats_unchanged() {
        assert_eq!(fmt("btc-usdt", CexExchange::Binance), "BTCUSDT");
        assert_eq!(fmt("ETHUSD", CexExchange::OKX), "ETH-USD");
        assert_eq!(fmt("BTCUSDT", CexExchange::Htx), "btcusdt");
        assert_eq!(fmt("BTCUSDT", CexExchange::Kraken), "XBTUSDT");
        // generic fallback for unknown quotes keeps the old behaviour
        assert_eq!(fmt("ETHEUR", CexExchange::OKX), "ETH-EUR");
        assert!(format_symbol_for_exchange("USDT", &CexExchange::OKX).is_err());
    }

    #[test]
    fn bitfinex_request_format() {
        assert_eq!(fmt("BTCUSDT", CexExchange::Bitfinex), "tBTCUST");
        assert_eq!(fmt("BTCUSD", CexExchange::Bitfinex), "tBTCUSD");
        // only the quote suffix is rewritten
        assert_eq!(fmt("USDTUSD", CexExchange::Bitfinex), "tUSDTUSD");
    }

    #[test]
    fn bitfinex_ws_symbols_are_standardised() {
        let std = |s| standard_symbol_for_cex_ws_response(s, &CexExchange::Bitfinex);
        assert_eq!(std("tBTCUST"), "BTCUSDT");
        assert_eq!(std("tETHUST"), "ETHUSDT");
        assert_eq!(std("tDOGE:UST"), "DOGEUSDT");
        assert_eq!(
            std("tTRXUST"),
            "TRXUSDT",
            "base starting with T keeps its T"
        );
        assert_eq!(std("tTONUST"), "TONUSDT");
        assert_eq!(std("tBTCUSD"), "BTCUSD");
        assert_eq!(std("tTESTBTC:TESTUSD"), "TESTBTCTESTUSD");
        // already-standard input is left alone
        assert_eq!(std("BTCUSDT"), "BTCUSDT");
        assert_eq!(bitfinex_symbol_to_standard("BTCUST"), "BTCUSDT");
    }

    #[test]
    fn json_ms_parses_numbers_strings_and_units() {
        use serde_json::json;
        assert_eq!(json_ms(Some(&json!(1794472001275u64))), Some(1794472001275));
        assert_eq!(json_ms(Some(&json!("1794472001275"))), Some(1794472001275));
        assert_eq!(
            json_ms(Some(&json!(1794472001u64))),
            Some(1794472001000),
            "seconds"
        );
        assert_eq!(
            json_ms(Some(&json!(1794472001275123u64))),
            Some(1794472001275),
            "micros"
        );
        assert_eq!(
            json_ms(Some(&json!(1794472001275123456u64))),
            Some(1794472001275),
            "nanos"
        );
        assert_eq!(json_ms(Some(&json!(0))), None);
        assert_eq!(json_ms(Some(&json!("abc"))), None);
        assert_eq!(json_ms(None), None);
    }

    #[test]
    fn iso8601_ms_parses_exchange_formats() {
        // Kraken / Coinbase style
        assert_eq!(
            iso8601_ms("2026-11-12T08:26:41.099248Z"),
            Some(1794472001099)
        );
        assert_eq!(iso8601_ms("2026-11-12T08:26:41Z"), Some(1794472001000));
        assert_eq!(
            iso8601_ms("2026-11-12T11:26:41.5+03:00"),
            Some(1794472001500)
        );
        assert_eq!(iso8601_ms("not a date"), None);
    }

    #[test]
    fn upbit_ws_symbols_are_standardised() {
        let std = |s| standard_symbol_for_cex_ws_response(s, &CexExchange::Upbit);
        assert_eq!(std("USDT-BTC"), "BTCUSDT");
        assert_eq!(std("KRW-OP"), "OPKRW");
    }
}
