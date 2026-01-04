mod types;

use crate::cex::kraken::types::KrakenDepthResponse;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const KRAKEN_API_BASE: &str = "https://api.kraken.com/0/public";

create_exchange!(Kraken);

#[async_trait]
impl ExchangeTrait for Kraken {
    fn api_base(&self) -> &str {
        KRAKEN_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Kraken"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Kraken time endpoint - test connectivity to the REST API
        let endpoint = "Time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Kraken returns {"error": [], "result": {"unixtime": ..., "rfc1123": ...}}
        let error = response["error"].as_array();
        if let Some(errors) = error {
            if errors.is_empty() && response["result"].is_object() {
                Ok(())
            } else {
                Err(MarketScannerError::HealthCheckFailed)
            }
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

        // Kraken uses special symbol format: BTCUSDT -> XBTUSDT
        // Convert BTC -> XBT for Kraken
        let kraken_symbol = if symbol.starts_with("BTC") {
            symbol.replace("BTC", "XBT").to_uppercase()
        } else {
            symbol.to_uppercase()
        };

        // Using Depth endpoint with count=1 for best bid/ask only
        let endpoint = format!("Depth?pair={}&count=1", kraken_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned errors
        let errors = response["error"].as_array().ok_or_else(|| {
            MarketScannerError::ApiError("Kraken API response missing error field".to_string())
        })?;

        if !errors.is_empty() {
            let error_msg = errors
                .iter()
                .filter_map(|e| e.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(MarketScannerError::ApiError(format!(
                "Kraken API error: {}",
                error_msg
            )));
        }

        // Deserialize response to KrakenDepthResponse
        let depth_response: KrakenDepthResponse =
            serde_json::from_value(response).map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Kraken API error: failed to parse depth response: {}",
                    e
                ))
            })?;

        // Get the first (and only) pair data from result
        let pair_data = depth_response.result.values().next().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no data found for symbol: {}",
                symbol
            ))
        })?;

        // Get best bid (first element in bids array: [price, quantity, timestamp])
        let bid_entry = pair_data.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity, timestamp])
        let ask_entry = pair_data.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        // Parse bid entry: [price, quantity, timestamp]
        let bid_price_str = bid_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid bid price format for symbol: {}",
                symbol
            ))
        })?;

        let bid_qty_str = bid_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid bid quantity format for symbol: {}",
                symbol
            ))
        })?;

        // Parse ask entry: [price, quantity, timestamp]
        let ask_price_str = ask_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid ask price format for symbol: {}",
                symbol
            ))
        })?;

        let ask_qty_str = ask_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Kraken API error: invalid ask quantity format for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(bid_price_str, "bid price")?;
        let ask = parse_f64(ask_price_str, "ask price")?;
        let bid_qty = parse_f64(bid_qty_str, "bid quantity")?;
        let ask_qty = parse_f64(ask_qty_str, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        Ok(CexPrice {
            symbol: symbol.to_uppercase(),
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Kraken),
        })
    }
}
