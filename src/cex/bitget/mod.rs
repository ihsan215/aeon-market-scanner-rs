mod types;

use crate::cex::bitget::types::BitgetOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const BITGET_API_BASE: &str = "https://api.bitget.com/api/v2";

create_exchange!(Bitget);

#[async_trait]
impl ExchangeTrait for Bitget {
    fn api_base(&self) -> &str {
        BITGET_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bitget"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bitget public server time endpoint - test connectivity to the REST API
        let endpoint = "public/time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Check if API returned success (Bitget uses "00000" for success)
        let code = response["code"].as_str();
        if code == Some("00000") {
            Ok(())
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for Bitget {
    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Bitget
        let bitget_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bitget)?;
        // Using v2 API orderbook endpoint (limit=1 for best bid/ask only)
        let endpoint = format!("spot/market/orderbook?symbol={}&limit=1", bitget_symbol);

        // First get as JSON value to check code
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success (Bitget uses "00000" for success)
        let code = response["code"].as_str().ok_or_else(|| {
            MarketScannerError::ApiError("Bitget API response missing code".to_string())
        })?;

        if code != "00000" {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Bitget API error: {} - {}",
                code, msg
            )));
        }

        // Deserialize response to BitgetOrderBookResponse using type definitions
        let orderbook_response: BitgetOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Bitget API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get data object
        let data = orderbook_response.data.ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: returned null or invalid data for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Bitget API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        let standard_symbol = crate::common::normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bitget),
        })
    }
}
