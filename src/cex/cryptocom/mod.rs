mod types;

use crate::cex::cryptocom::types::CryptocomOrderBookResponse;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, normalize_symbol, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

// Crypto.com Exchange API base URL
// Note: API base URL might need to be verified
const CRYPTOCOM_API_BASE: &str = "https://api.crypto.com/v2/public";

create_exchange!(Cryptocom);

#[async_trait]
impl ExchangeTrait for Cryptocom {
    fn api_base(&self) -> &str {
        CRYPTOCOM_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Crypto.com"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Crypto.com Exchange book endpoint - test connectivity with BTC_USDT
        // Time endpoint returns BAD_REQUEST, so we use get-book instead
        // Note: api_base already includes /public, so we don't need to prefix with "public/"
        let endpoint = "get-book?instrument_name=BTC_USDT&depth=1";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Check if response indicates successful connection
        // Crypto.com returns {"code": 0, "result": {...}}
        if let Some(code) = response.get("code") {
            if code.as_i64() == Some(0) {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Crypto.com Exchange
        let cryptocom_symbol = format_symbol_for_exchange(symbol, &CexExchange::Cryptocom)?;

        // Get orderbook
        // Note: api_base already includes /public, so we don't need to prefix with "public/"
        let endpoint = format!("get-book?instrument_name={}&depth=1", cryptocom_symbol);

        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check for errors in response
        if let Some(code) = response.get("code") {
            if code.as_i64() != Some(0) {
                if let Some(msg) = response.get("message") {
                    return Err(MarketScannerError::ApiError(format!(
                        "Crypto.com API error: {} - {}",
                        code, msg
                    )));
                }
            }
        }

        // Parse orderbook response
        let orderbook_response: CryptocomOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Crypto.com API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get first data entry (should be for the requested symbol)
        let orderbook_data = orderbook_response.result.data.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no orderbook data found for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid and ask
        let bid_entry = orderbook_data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        let ask_entry = orderbook_data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Crypto.com API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);
        let standard_symbol = normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Cryptocom),
        })
    }
}

