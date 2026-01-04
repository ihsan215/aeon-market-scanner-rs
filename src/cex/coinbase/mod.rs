mod types;

use crate::cex::coinbase::types::CoinbaseOrderBookResponse;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const COINBASE_API_BASE: &str = "https://api.exchange.coinbase.com";

create_exchange!(Coinbase);

#[async_trait]
impl ExchangeTrait for Coinbase {
    fn api_base(&self) -> &str {
        COINBASE_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Coinbase"
    }

    // Override get method to add User-Agent header
    async fn get<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
    ) -> Result<T, MarketScannerError> {
        let url = format!("{}/{}", self.api_base(), endpoint);
        let response = self
            .client()
            .get(&url)
            .header("User-Agent", "aeon-market-scanner-rs")
            .send()
            .await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            // For 404, include the endpoint in error message
            if status == 404 {
                return Err(MarketScannerError::ApiError(format!(
                    "{} API error: {} - {} (endpoint: {})",
                    self.exchange_name(),
                    status,
                    error_text,
                    endpoint
                )));
            }
            return Err(MarketScannerError::ApiError(format!(
                "{} API error: {} - {}",
                self.exchange_name(),
                status,
                error_text
            )));
        }

        Ok(response.json().await?)
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Coinbase time endpoint - test connectivity to the REST API
        let endpoint = "time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Coinbase returns {"iso": "...", "epoch": ...}
        if response["iso"].is_string() {
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

        // Coinbase uses format: BTC-USD or BTC-USDT (with dash)
        // Convert BTCUSDT -> BTC-USDT, BTCUSD -> BTC-USD
        let coinbase_symbol = if symbol.contains('-') {
            symbol.to_uppercase()
        } else {
            let symbol_upper = symbol.to_uppercase();
            // Find split point: assume last 3-4 chars are quote currency
            let split_point = if symbol_upper.len() >= 7 && symbol_upper.ends_with("USDT") {
                symbol.len() - 4
            } else if symbol_upper.len() >= 6 && symbol_upper.ends_with("USD") {
                symbol.len() - 3
            } else if symbol.len() >= 6 {
                symbol.len() - 3
            } else {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Invalid symbol format: {}",
                    symbol
                )));
            };
            format!("{}-{}", &symbol[..split_point], &symbol[split_point..]).to_uppercase()
        };

        // Using orderbook endpoint with level=1 for best bid/ask only
        let endpoint = format!("products/{}/book?level=1", coinbase_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response has error
        if let Some(message) = response.get("message") {
            if message.as_str() == Some("NotFound") {
                return Err(MarketScannerError::ApiError(format!(
                    "Coinbase API error: symbol {} not found (tried endpoint: products/{}/book?level=1)",
                    symbol, coinbase_symbol
                )));
            }
        }

        // Deserialize response directly to CoinbaseOrderBookResponse
        let orderbook_response: CoinbaseOrderBookResponse = serde_json::from_value(response)
            .map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "Coinbase API error: failed to parse orderbook response: {}",
                    e
                ))
            })?;

        // Get best bid (first element in bids array: [price, quantity, order_count])
        let bid_entry = orderbook_response.bids.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: no bid found for symbol: {}",
                symbol
            ))
        })?;

        // Get best ask (first element in asks array: [price, quantity, order_count])
        let ask_entry = orderbook_response.asks.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: no ask found for symbol: {}",
                symbol
            ))
        })?;

        // Parse bid entry: [price, quantity, order_count]
        let bid_price_str = bid_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid bid price format for symbol: {}",
                symbol
            ))
        })?;

        let bid_qty_str = bid_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid bid quantity format for symbol: {}",
                symbol
            ))
        })?;

        // Parse ask entry: [price, quantity, order_count]
        let ask_price_str = ask_entry[0].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid ask price format for symbol: {}",
                symbol
            ))
        })?;

        let ask_qty_str = ask_entry[1].as_str().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Coinbase API error: invalid ask quantity format for symbol: {}",
                symbol
            ))
        })?;

        let bid = parse_f64(bid_price_str, "bid price")?;
        let ask = parse_f64(ask_price_str, "ask price")?;
        let bid_qty = parse_f64(bid_qty_str, "bid quantity")?;
        let ask_qty = parse_f64(ask_qty_str, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert Coinbase symbol format (BTC-USDT) to standard (BTCUSDT)
        let standard_symbol = coinbase_symbol.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Coinbase),
        })
    }
}
