mod types;

use crate::cex::htx::types::HtxOrderBookResponse;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis,
};
use crate::create_exchange;
use async_trait::async_trait;

const HTX_API_BASE: &str = "https://api.htx.com";

create_exchange!(Htx);

#[async_trait]
impl ExchangeTrait for Htx {
    fn api_base(&self) -> &str {
        HTX_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "HTX"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // HTX orderbook endpoint - test connectivity to the REST API
        // Using a common pair like BTCUSDT for health check
        let endpoint = "market/depth?symbol=btcusdt&type=step0";
        let response: serde_json::Value = self.get(endpoint).await?;

        // HTX returns {"status": "ok", ...}
        let status = response["status"].as_str().unwrap_or("");
        if status == "ok" {
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

        // Format symbol for HTX
        let htx_symbol = format_symbol_for_exchange(symbol, &CexExchange::Htx)?;
        let endpoint = format!("market/depth?symbol={}&type=step0", htx_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success
        let status = response["status"].as_str().unwrap_or("");
        if status != "ok" {
            let err_msg = response["err-msg"]
                .as_str()
                .unwrap_or("Unknown error")
                .to_string();
            return Err(MarketScannerError::ApiError(format!(
                "HTX API error: {}",
                err_msg
            )));
        }

        // Deserialize response to HtxOrderBookResponse
        let orderbook_response: HtxOrderBookResponse =
            serde_json::from_value(response).map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "HTX API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = orderbook_response.tick.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "HTX API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = orderbook_response.tick.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "HTX API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        // HTX returns numbers directly, not strings
        let bid = bid_entry[0];
        let ask = ask_entry[0];
        let bid_qty = bid_entry[1];
        let ask_qty = ask_entry[1];

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
            exchange: Exchange::Cex(CexExchange::Htx),
        })
    }
}

