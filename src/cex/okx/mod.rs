mod types;

use crate::cex::okx::types::OkxTickerResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const OKX_API_BASE: &str = "https://www.okx.com/api/v5";

create_exchange!(OKX);

#[async_trait]
impl ExchangeTrait for OKX {
    fn api_base(&self) -> &str {
        OKX_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "OKX"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // OKX public/time endpoint - returns server time
        let endpoint = "public/time";
        let response: serde_json::Value = self.get(endpoint).await?;

        // OKX returns {"code":"0", "data":[...], "msg":""} for success
        if let Some(code) = response["code"].as_str() {
            if code == "0" {
                Ok(())
            } else {
                let msg = response["msg"].as_str().unwrap_or("Unknown error");
                Err(MarketScannerError::ApiError(format!(
                    "OKX health check failed: {} - {}",
                    code, msg
                )))
            }
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

#[async_trait]
impl CEXTrait for OKX {
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
        // Format symbol for OKX
        let okx_symbol = format_symbol_for_exchange(symbol, &CexExchange::OKX)?;
        let endpoint = format!("market/ticker?instId={}", okx_symbol);

        let response: OkxTickerResponse = self.get(&endpoint).await?;

        // Check if API returned success
        if response.code != "0" {
            return Err(MarketScannerError::ApiError(format!(
                "OKX API error: {} - {}",
                response.code, response.msg
            )));
        }

        // Get first ticker data
        let ticker = response.data.first().ok_or_else(|| {
            MarketScannerError::ApiError("OKX API returned empty data".to_string())
        })?;

        let bid = parse_f64(&ticker.bid_px, "bid price")?;
        let ask = parse_f64(&ticker.ask_px, "ask price")?;
        let bid_qty = parse_f64(&ticker.bid_sz, "bid quantity")?;
        let ask_qty = parse_f64(&ticker.ask_sz, "ask quantity")?;
        let mid_price = find_mid_price(bid, ask);

        // Convert OKX symbol format (BTC-USDT) to standard (BTCUSDT)
        let standard_symbol = ticker.inst_id.replace("-", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::OKX),
        })
    }
}
