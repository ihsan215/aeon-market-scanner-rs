mod types;

use crate::cex::bitfinex::types::BitfinexOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, normalize_symbol,
};
use crate::create_exchange;
use async_trait::async_trait;

const BITFINEX_API_BASE: &str = "https://api-pub.bitfinex.com/v2";

create_exchange!(Bitfinex);

#[async_trait]
impl ExchangeTrait for Bitfinex {
    fn api_base(&self) -> &str {
        BITFINEX_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bitfinex"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bitfinex platform status endpoint - test connectivity to the REST API
        let endpoint = "platform/status";
        let response: types::BitfinexPlatformStatus = self.get(endpoint).await?;

        // Bitfinex returns [1] for operational, [0] for maintenance
        if let Some(code) = response.first() {
            if *code == 1 {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }
}

#[async_trait]
impl CEXTrait for Bitfinex {
    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Bitfinex (tBTCUSD format)
        let bitfinex_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bitfinex)?;

        // Using orderbook endpoint with P0 precision and len=1 for best bid/ask only
        let endpoint = format!("book/{}/P0?len=1", bitfinex_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response is an error array (Bitfinex v2 returns errors as [error_code, "error_message"])
        if let Some(array) = response.as_array() {
            if array.len() == 2 {
                if let (Some(code), Some(msg)) = (
                    array.get(0).and_then(|v| v.as_i64()),
                    array.get(1).and_then(|v| v.as_str()),
                ) {
                    if code != 0 {
                        return Err(MarketScannerError::ApiError(format!(
                            "Bitfinex API error: {} - {}",
                            code, msg
                        )));
                    }
                }
            }
        }

        // Deserialize response to BitfinexOrderBookResponse
        // Bitfinex returns orderbook as array: [[price, count, amount], ...]
        let orderbook_response: BitfinexOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Separate bids (negative amount) and asks (positive amount)
        // Bitfinex: amount < 0 means bid (buy order), amount > 0 means ask (sell order)
        let mut bids: Vec<(f64, f64)> = Vec::new();
        let mut asks: Vec<(f64, f64)> = Vec::new();

        for entry in orderbook_response {
            let price = entry[0];
            let _count = entry[1] as i64;
            let amount = entry[2];

            if amount < 0.0 {
                // Bid (negative amount) - buyers want to buy at this price
                bids.push((price, amount.abs()));
            } else if amount > 0.0 {
                // Ask (positive amount) - sellers want to sell at this price
                asks.push((price, amount));
            }
        }

        // Get best bid (highest bid price - buyers want highest price they're willing to pay)
        let bid_entry = bids
            .iter()
            .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: no bid found for symbol: {}",
                    symbol
                ))
            })?;

        // Get best ask (lowest ask price - sellers want lowest price they're willing to accept)
        let ask_entry = asks
            .iter()
            .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
            .ok_or_else(|| {
                MarketScannerError::ApiError(format!(
                    "Bitfinex API error: no ask found for symbol: {}",
                    symbol
                ))
            })?;

        let mut bid = bid_entry.0;
        let mut ask = ask_entry.0;
        let mut bid_qty = bid_entry.1;
        let mut ask_qty = ask_entry.1;

        // Ensure bid <= ask (if not, swap them as this shouldn't happen but Bitfinex might return them reversed)
        if bid > ask {
            std::mem::swap(&mut bid, &mut ask);
            std::mem::swap(&mut bid_qty, &mut ask_qty);
        }

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        // Bitfinex converts USDT to UST, so we need to convert back
        // But we should preserve what was actually used on the exchange
        // Since we converted BTCUSDT -> tBTCUST, we should return BTCUST in the response
        let standard_symbol = if symbol.to_uppercase().ends_with("USDT") {
            // Convert back: BTCUSDT -> BTCUST (what Bitfinex actually uses)
            let base = symbol
                .to_uppercase()
                .replace("-", "")
                .replace("_", "")
                .replace("USDT", "UST");
            base
        } else {
            normalize_symbol(symbol)
        };

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bitfinex),
        })
    }
}
