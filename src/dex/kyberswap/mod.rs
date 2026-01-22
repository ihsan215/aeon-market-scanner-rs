mod types;
mod utils;

use crate::common::{
    DEXTrait, DexAggregator, DexPrice, DexRouteSummary, Exchange, ExchangeTrait,
    MarketScannerError, find_mid_price, get_timestamp_millis,
};
use crate::create_exchange;
use async_trait::async_trait;
use types::KyberSwapRoutesResponse;
use utils::{calculate_amount_for_value, create_http_client_with_browser_headers, wei_to_eth};

const KYBERSWAP_API_BASE: &str = "https://aggregator-api.kyberswap.com";

create_exchange!(KyberSwap);

#[async_trait]
impl ExchangeTrait for KyberSwap {
    fn api_base(&self) -> &str {
        KYBERSWAP_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "KyberSwap"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // KyberSwap doesn't have a ping endpoint, so we test with a simple route query
        // Use Ethereum mainnet as the default chain for health check
        let chain_name = "ethereum";
        let api_base = format!("{}/{}/api/v1", KYBERSWAP_API_BASE, chain_name);

        // Test with a known token pair on Ethereum (ETH -> USDT)
        let url = format!(
            "{}/routes?tokenIn=0xEeeeeEeeeEeEeeEeEeEeeEEEeeeeEeeeeeeeEEeE&tokenOut=0xdAC17F958D2ee523a2206206994597C13D831ec7&amountIn=1000000000000000&gasInclude=true",
            api_base
        );

        // Build client with custom headers to bypass Cloudflare protection
        let client = create_http_client_with_browser_headers()?;

        let response = client
            .get(&url)
            .send()
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(MarketScannerError::HealthCheckFailed)
        }
    }
}

//TODO: add qutoto amount in params
//TODO: find ask price for selling quote token for base token
//TODO: find bid price for buying base token with quote token use ask ratio for determine amount
//TODO: unifed response and return
#[async_trait]
impl DEXTrait for KyberSwap {
    async fn get_price(
        &self,
        base_token: &crate::dex::chains::Token,
        quote_token: &crate::dex::chains::Token,
        quote_amount: f64,
    ) -> Result<DexPrice, MarketScannerError> {
        // Validate that both tokens are on the same chain
        if base_token.chain_id != quote_token.chain_id {
            return Err(MarketScannerError::InvalidSymbol(format!(
                "Base token and quote token must be on the same chain. Base: {:?}, Quote: {:?}",
                base_token.chain_id, quote_token.chain_id
            )));
        }

        let quote_amount_str = calculate_amount_for_value(quote_amount, quote_token.decimal);

        // Get chain-specific API base URL from token's chain_id
        let chain_name = base_token.chain_id.name();
        let api_base = format!("{}/{}/api/v1", KYBERSWAP_API_BASE, chain_name);

        // Create symbol from token symbols (for DexPrice)
        let normalized = format!("{}{}", base_token.symbol, quote_token.symbol);

        // Build client with custom headers to bypass Cloudflare protection
        let client = create_http_client_with_browser_headers()?;

        // First Calculate Bid price (quote token -> base token)
        let bid_endpoint = format!(
            "{}/routes?tokenIn={}&tokenOut={}&amountIn={}&gasInclude=true&saveGas=0&excludedSources=bebop,smardex,dodo",
            api_base, quote_token.address, base_token.address, quote_amount_str
        );

        let bid_response_raw = client
            .get(&bid_endpoint)
            .send()
            .await
            .map_err(|e| MarketScannerError::HttpError(e))?;

        let status = bid_response_raw.status();
        if !status.is_success() {
            let error_text = bid_response_raw.text().await.unwrap_or_default();
            return Err(MarketScannerError::ApiError(format!(
                "KyberSwap API error: status {} - {}",
                status, error_text
            )));
        }

        let bid_response: KyberSwapRoutesResponse = bid_response_raw.json().await.map_err(|e| {
            MarketScannerError::ApiError(format!("Failed to parse KyberSwap response: {}", e))
        })?;

        if bid_response.code != 0 {
            return Err(MarketScannerError::ApiError(format!(
                "KyberSwap API error: {}",
                bid_response.message.unwrap_or_default()
            )));
        }

        let bid_data = bid_response.data.ok_or_else(|| {
            MarketScannerError::ApiError("KyberSwap API returned no data".to_string())
        })?;

        // Parse amounts using safe conversion with Decimal for precision
        let bid_amount_in_decimal =
            wei_to_eth(&bid_data.route_summary.amount_in, quote_token.decimal)?;
        let bid_amount_out_decimal =
            wei_to_eth(&bid_data.route_summary.amount_out, base_token.decimal)?;
        // Price per 1 base token in  (quote token)
        let bid_price = utils::safe_divide(bid_amount_in_decimal, bid_amount_out_decimal)?;

        let bid_route_summary = DexRouteSummary {
            token_in: bid_data.route_summary.token_in.clone(),
            token_out: bid_data.route_summary.token_out.clone(),
            amount_in: bid_data.route_summary.amount_in.clone(),
            amount_out: bid_data.route_summary.amount_out.clone(),
            amount_in_wei: wei_to_eth(&bid_data.route_summary.amount_in, quote_token.decimal)?,
            amount_out_wei: wei_to_eth(&bid_data.route_summary.amount_out, base_token.decimal)?,
        };

        let bid_route_data = serde_json::to_value(&bid_data).ok();

        // Query for ASK price: selling base token for quote token (base -> quote)
        // Use the raw amount_out from bid response (already in raw format with decimals)
        let ask_endpoint = format!(
            "{}/routes?tokenIn={}&tokenOut={}&amountIn={}&gasInclude=true&saveGas=0&excludedSources=bebop,smardex,dodo",
            api_base, base_token.address, quote_token.address, bid_data.route_summary.amount_out
        );

        let ask_response_raw = client
            .get(&ask_endpoint)
            .send()
            .await
            .map_err(|e| MarketScannerError::HttpError(e))?;

        let status = ask_response_raw.status();
        if !status.is_success() {
            let error_text = ask_response_raw.text().await.unwrap_or_default();
            return Err(MarketScannerError::ApiError(format!(
                "KyberSwap API error: status {} - {}",
                status, error_text
            )));
        }

        let ask_response: KyberSwapRoutesResponse = ask_response_raw.json().await.map_err(|e| {
            MarketScannerError::ApiError(format!("Failed to parse KyberSwap response: {}", e))
        })?;

        if ask_response.code != 0 {
            return Err(MarketScannerError::ApiError(format!(
                "KyberSwap API error: {}",
                ask_response.message.unwrap_or_default()
            )));
        }

        let ask_data = ask_response.data.ok_or_else(|| {
            MarketScannerError::ApiError("KyberSwap API returned no data".to_string())
        })?;

        // Parse amounts using safe conversion with Decimal for precision
        let ask_amount_in_decimal =
            wei_to_eth(&ask_data.route_summary.amount_in, base_token.decimal)?;
        let ask_amount_out_decimal =
            wei_to_eth(&ask_data.route_summary.amount_out, quote_token.decimal)?;
        let ask_price = utils::safe_divide(ask_amount_out_decimal, ask_amount_in_decimal)?;

        // Store route summary for ask
        let ask_route_summary = DexRouteSummary {
            token_in: ask_data.route_summary.token_in.clone(),
            token_out: ask_data.route_summary.token_out.clone(),
            amount_in: ask_data.route_summary.amount_in.clone(),
            amount_out: ask_data.route_summary.amount_out.clone(),
            amount_in_wei: wei_to_eth(&ask_data.route_summary.amount_in, base_token.decimal)?,
            amount_out_wei: wei_to_eth(&ask_data.route_summary.amount_out, quote_token.decimal)?,
        };

        // Store full route data as JSON
        let ask_route_data = serde_json::to_value(&ask_data).ok();

        let mid_price = find_mid_price(bid_price, ask_price);

        // Calculate quantities using safe conversion
        let bid_qty = wei_to_eth(&bid_data.route_summary.amount_out, base_token.decimal)?;
        let ask_qty = wei_to_eth(&ask_data.route_summary.amount_in, base_token.decimal)?;

        Ok(DexPrice {
            symbol: normalized,
            mid_price,
            bid_price: bid_price,
            ask_price: ask_price,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Dex(DexAggregator::KyberSwap),
            bid_route_summary: Some(bid_route_summary),
            ask_route_summary: Some(ask_route_summary),
            bid_route_data: bid_route_data,
            ask_route_data: ask_route_data,
        })
    }
}
