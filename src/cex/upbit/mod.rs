mod types;

use crate::cex::upbit::types::UpbitOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, normalize_symbol,
};
use crate::create_exchange;
use async_trait::async_trait;

const UPBIT_API_BASE: &str = "https://api.upbit.com/v1";

create_exchange!(Upbit);

#[async_trait]
impl ExchangeTrait for Upbit {
    fn api_base(&self) -> &str {
        UPBIT_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Upbit"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Upbit market all endpoint - test connectivity to the REST API
        let endpoint = "market/all?isDetails=false";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Upbit returns array of market objects for success
        if let Some(array) = response.as_array() {
            if !array.is_empty() {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }
}

#[async_trait]
impl CEXTrait for Upbit {
    fn supports_websocket(&self) -> bool {
        false
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Upbit (KRW-BTC format)
        let upbit_symbol = format_symbol_for_exchange(symbol, &CexExchange::Upbit)?;

        // Using orderbook endpoint
        let endpoint = format!("orderbook?markets={}", upbit_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response is an error
        if let Some(error) = response.get("error") {
            let error_msg = error.as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Upbit API error: {}",
                error_msg
            )));
        }

        // Deserialize response to UpbitOrderBookResponse (it's an array with one element)
        let orderbook_array = response.as_array().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: invalid orderbook response format for symbol: {}",
                symbol
            ))
        })?;

        let orderbook_response: UpbitOrderBookResponse = serde_json::from_value(
            orderbook_array
                .first()
                .ok_or_else(|| {
                    MarketScannerError::ApiError(format!(
                        "Upbit API error: empty orderbook response for symbol: {}",
                        symbol
                    ))
                })?
                .clone(),
        )
        .map_err(|e| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: failed to parse orderbook response: {}",
                e
            ))
        })?;

        // Get best bid and ask from first orderbook unit
        let best_unit = orderbook_response.orderbook_units.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: no orderbook units found for symbol: {}",
                symbol
            ))
        })?;

        let bid = best_unit.bid_price;
        let ask = best_unit.ask_price;
        let bid_qty = best_unit.bid_size;
        let ask_qty = best_unit.ask_size;

        // Ensure bid <= ask
        let (bid, ask, bid_qty, ask_qty) = if bid > ask {
            (ask, bid, ask_qty, bid_qty)
        } else {
            (bid, ask, bid_qty, ask_qty)
        };

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        let standard_symbol = normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Upbit),
        })
    }
}
