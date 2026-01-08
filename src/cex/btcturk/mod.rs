mod types;

use crate::cex::btcturk::types::BtcturkOrderBookResponse;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const BTCTURK_API_BASE: &str = "https://api.btcturk.com/api/v2";

create_exchange!(Btcturk);

#[async_trait]
impl ExchangeTrait for Btcturk {
    fn api_base(&self) -> &str {
        BTCTURK_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "BTCTurk"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // BTCTurk orderbook endpoint - test connectivity to the REST API
        // Using a common pair like BTCUSDT for health check
        let endpoint = "orderbook?pairSymbol=BTCUSDT&limit=1";
        let response: serde_json::Value = self.get(endpoint).await?;

        // BTCTurk returns {"data": {...}, "success": true, ...}
        let success = response["success"].as_bool().unwrap_or(false);
        if success {
            Ok(())
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for BTCTurk
        let btcturk_symbol = format_symbol_for_exchange(symbol, &CexExchange::Btcturk)?;
        // Using orderbook endpoint to get both prices and quantities
        let endpoint = format!("orderbook?pairSymbol={}&limit=1", btcturk_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success
        let success = response["success"].as_bool().unwrap_or(false);
        if !success {
            let message = response["message"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            return Err(MarketScannerError::ApiError(format!(
                "BTCTurk API error: {}",
                message
            )));
        }

        // Deserialize response to BtcturkOrderBookResponse
        let orderbook_response: BtcturkOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "BTCTurk API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = orderbook_response.data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "BTCTurk API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = orderbook_response.data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "BTCTurk API error: no ask found for symbol: {}",
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
            exchange: Exchange::Cex(CexExchange::Btcturk),
        })
    }
}
