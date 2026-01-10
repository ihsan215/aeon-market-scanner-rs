mod types;

use crate::cex::bybit::types::BybitTickerData;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, normalize_symbol, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const BYBIT_API_BASE: &str = "https://api.bybit.com/v5";

create_exchange!(Bybit);

#[async_trait]
impl ExchangeTrait for Bybit {
    fn api_base(&self) -> &str {
        BYBIT_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Bybit"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Bybit market/time endpoint - test connectivity to the REST API
        let endpoint = "market/time";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Bybit {
    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }
        // Format symbol for Bybit
        let bybit_symbol = format_symbol_for_exchange(symbol, &CexExchange::Bybit)?;
        let endpoint = format!("market/tickers?category=spot&symbol={}", bybit_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if API returned success (Bybit uses camelCase in JSON)
        let ret_code = response["retCode"].as_i64().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API response missing retCode".to_string())
        })?;

        if ret_code != 0 {
            let ret_msg = response["retMsg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Bybit API error: {} - {}",
                ret_code, ret_msg
            )));
        }

        // Parse the result.list array
        let list = response["result"]["list"].as_array().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API returned invalid data format".to_string())
        })?;

        let ticker_value = list.first().ok_or_else(|| {
            MarketScannerError::ApiError("Bybit API returned empty data".to_string())
        })?;

        // Deserialize the ticker data
        let ticker: BybitTickerData =
            serde_json::from_value(ticker_value.clone()).map_err(|e| {
                MarketScannerError::ApiError(format!("Failed to parse Bybit ticker data: {}", e))
            })?;

        let bid = parse_f64(&ticker.bid1_price, "bid price")?;
        let ask = parse_f64(&ticker.ask1_price, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid1_size, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask1_size, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol to standard format
        let standard_symbol = normalize_symbol(&ticker.symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Bybit),
        })
    }
}
