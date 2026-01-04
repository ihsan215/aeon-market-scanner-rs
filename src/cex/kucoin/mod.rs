mod types;
use crate::common::{
    CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const KUCOIN_API_BASE: &str = "https://api.kucoin.com/api/v1";

create_exchange!(Kucoin);

#[async_trait]
impl ExchangeTrait for Kucoin {
    fn api_base(&self) -> &str {
        KUCOIN_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "KuCoin"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // KuCoin timestamp endpoint - test connectivity to the REST API
        let endpoint = "timestamp";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // KuCoin uses format: BTC-USDT (with dash, similar to OKX)
        // Convert BTCUSDT -> BTC-USDT
        // For invalid symbols, let the API handle the error
        let kucoin_symbol = if symbol.contains('-') {
            symbol.to_uppercase()
        } else {
            // Try to split symbol, but don't fail if format is invalid
            // Let API return error for invalid symbols
            let split_point = if symbol.len() >= 7 && symbol.ends_with("USDT") {
                symbol.len() - 4
            } else if symbol.len() >= 6 {
                symbol.len() - 3
            } else {
                // For very short symbols, try to split at middle
                // This will likely result in API error, which is fine
                symbol.len() / 2
            };
            format!("{}-{}", &symbol[..split_point], &symbol[split_point..]).to_uppercase()
        };

        // Get order book level 1 for bid/ask prices and quantities
        let book_endpoint = format!("market/orderbook/level1?symbol={}", kucoin_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&book_endpoint).await?;

        // Check if API returned success (KuCoin uses "200000" for success)
        let code = response["code"].as_str().ok_or_else(|| {
            MarketScannerError::ApiError("KuCoin API response missing code".to_string())
        })?;

        if code != "200000" {
            let msg = response["msg"].as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "KuCoin API error: {} - {}",
                code, msg
            )));
        }

        // Check if data exists
        let data = response["data"].as_object().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "KuCoin API error: returned null or invalid data for symbol: {}",
                symbol
            ))
        })?;
        // Deserialize the order book data
        let order_book_data: types::KucoinOrderBookData =
            serde_json::from_value(serde_json::Value::Object(data.clone())).map_err(|e| {
                MarketScannerError::ApiError(format!(
                    "KuCoin API error: failed to parse order book data: {}",
                    e
                ))
            })?;
        // Get best bid and ask from order book data
        let bid = parse_f64(&order_book_data.best_bid, "bid price")?;
        let ask = parse_f64(&order_book_data.best_ask, "ask price")?;
        let bid_qty = parse_f64(&order_book_data.best_bid_size, "bid quantity")?;
        let ask_qty = parse_f64(&order_book_data.best_ask_size, "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert KuCoin symbol format (BTC-USDT) to standard (BTCUSDT)
        let standard_symbol = kucoin_symbol.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Kucoin),
        })
    }
}
