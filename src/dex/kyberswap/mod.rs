mod types;
mod utils;

use crate::common::{
    DEXTrait, DexAggregator, DexPrice, DexRouteSummary, Exchange, ExchangeTrait,
    MarketScannerError, find_mid_price, get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;
use types::KyberSwapRoutesResponse;
use utils::{calculate_amount_for_value, create_http_client_with_browser_headers};

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
    ) -> Result<DexPrice, MarketScannerError> {
        // Validate that both tokens are on the same chain
        if base_token.chain_id != quote_token.chain_id {
            return Err(MarketScannerError::InvalidSymbol(format!(
                "Base token and quote token must be on the same chain. Base: {:?}, Quote: {:?}",
                base_token.chain_id, quote_token.chain_id
            )));
        }

        // Convert $1000 USD to token amount (using quote token decimals)
        // For $1000: 1000 * 10^decimals
        let usd_amount = 1000.0;
        let quote_amount_str = calculate_amount_for_value(usd_amount, quote_token.decimal);

        // Get chain-specific API base URL from token's chain_id
        let chain_name = base_token.chain_id.name();
        let api_base = format!("{}/{}/api/v1", KYBERSWAP_API_BASE, chain_name);

        // Create symbol from token symbols (for DexPrice)
        let normalized = format!("{}{}", base_token.symbol, quote_token.symbol);

        // Build client with custom headers to bypass Cloudflare protection
        let client = create_http_client_with_browser_headers()?;

        // Query for ASK price: selling base token for quote token (base -> quote)
        // This gives us the price when selling base token (ask)
        let ask_endpoint = format!(
            "{}/routes?tokenIn={}&tokenOut={}&amountIn={}&gasInclude=true&saveGas=0&excludedSources=bebop,smardex,dodo",
            api_base,
            base_token.address,
            quote_token.address,
            // Use $1000 worth of base token - need to calculate based on base decimals
            // For simplicity, use quote amount but we should ideally convert to base amount
            // For now, we'll use an estimated amount or get a quote first
            calculate_amount_for_value(usd_amount, base_token.decimal)
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

        // Parse amounts
        let ask_amount_in = parse_f64(&ask_data.route_summary.amount_in, "amount in")?;
        let ask_amount_out = parse_f64(&ask_data.route_summary.amount_out, "amount out")?;

        // Calculate ask price: base token price in USD (when selling base token)
        // ask_amount_out is in quote token (USDT/USDC), convert to USD value per base token
        // Formula: (quote token received / quote decimals) / (base token sold / base decimals)
        let ask_amount_in_decimal = ask_amount_in / 10_f64.powi(base_token.decimal as i32);
        let ask_amount_out_decimal = ask_amount_out / 10_f64.powi(quote_token.decimal as i32);
        // Price per 1 base token in USD (quote token)
        let ask_price = ask_amount_out_decimal / ask_amount_in_decimal;

        // Store route summary for ask
        let ask_route_summary = DexRouteSummary {
            token_in: ask_data.route_summary.token_in.clone(),
            token_out: ask_data.route_summary.token_out.clone(),
            amount_in: ask_data.route_summary.amount_in.clone(),
            amount_out: ask_data.route_summary.amount_out.clone(),
        };

        // Store full route data as JSON
        let ask_route_data = serde_json::to_value(&ask_data).ok();

        // Query for BID price: buying base token with quote token (quote -> base)
        // This gives us the price when buying base token (bid)
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

        // Parse amounts
        let bid_amount_in = parse_f64(&bid_data.route_summary.amount_in, "amount in")?;
        let bid_amount_out = parse_f64(&bid_data.route_summary.amount_out, "amount out")?;

        // Calculate bid price: base token price in USD (when buying base token)
        // bid_amount_in is in quote token (USDT/USDC), convert to USD value per base token
        // Formula: (quote token spent / quote decimals) / (base token received / base decimals)
        let bid_amount_in_decimal = bid_amount_in / 10_f64.powi(quote_token.decimal as i32);
        let bid_amount_out_decimal = bid_amount_out / 10_f64.powi(base_token.decimal as i32);
        // Price per 1 base token in USD (quote token)
        let bid_price = bid_amount_in_decimal / bid_amount_out_decimal;

        // Store route summary for bid
        let bid_route_summary = DexRouteSummary {
            token_in: bid_data.route_summary.token_in.clone(),
            token_out: bid_data.route_summary.token_out.clone(),
            amount_in: bid_data.route_summary.amount_in.clone(),
            amount_out: bid_data.route_summary.amount_out.clone(),
        };

        // Store full route data as JSON
        let bid_route_data = serde_json::to_value(&bid_data).ok();

        let mid_price = find_mid_price(bid_price, ask_price);

        // Calculate quantities (using the amounts from quotes)
        let bid_qty = bid_amount_out / 10_f64.powi(base_token.decimal as i32);
        let ask_qty = ask_amount_in / 10_f64.powi(base_token.decimal as i32);

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

// use crate::common::ExchangeTrait;
// use crate::create_exchange;
// use async_trait::async_trait;

// const KYBERSWAP_API_BASE: &str = "https://aggregator-api.kyberswap.com";
//
// create_exchange!(Kyberswap);

// TODO: Added chain names ->
/*

  private getApiBaseUrl(chainId: ChainId): string {
    const chainNames = {
      [ChainId.ETHEREUM]: 'ethereum',
      [ChainId.BSC]: 'bsc',
      [ChainId.POLYGON]: 'polygon',
      [ChainId.AVALANCHE]: 'avalanche',
      [ChainId.ARBITRUM]: 'arbitrum',
      [ChainId.OPTIMISM]: 'optimism',
      [ChainId.BASE]: 'base',
      [ChainId.PLASMA]: 'plasma',
      [ChainId.UNICHAIN]: 'unichain',
      [ChainId.SONIC]: 'sonic',
      [ChainId.RONIN]: 'ronin',
      [ChainId.HyperEVM]: 'hyprevm',
      [ChainId.LINEA]: 'linea',
      [ChainId.MANTLE]: 'mantle',
    };

    const chainName = chainNames[chainId] || 'ethereum';
    return `https://aggregator-api.kyberswap.com/${chainName}/api/v1`;
  }


  // get route
        // Step 1: Get the best route using a GET request
      const routeParams = {
        tokenIn: fromToken,
        tokenOut: toToken,
        amountIn: amount,
        gasInclude: true,
        saveGas: 0,
        excludedSources: 'bebop,smardex,dodo', // Exclude problematic sources
      };

      const apiConfig = {
        headers: { 'X-Client-Id': 'wc-arbitrage-bot' },
      };
      const routeResponse = await firstValueFrom(
        this.httpService.get(`${baseUrl}/routes`, {
          params: routeParams,
          ...apiConfig,
        }),
      );


*/

// mod types;
// use crate::common::{
//     CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
//     format_symbol_for_exchange, get_timestamp_millis, normalize_symbol, parse_f64,
// };
// use crate::create_exchange;
// use async_trait::async_trait;
// use types::BinanceBookTickerResponse;

// const BINANCE_API_BASE: &str = "https://api.binance.com/api/v3";

// create_exchange!(Binance);

// #[async_trait]
// impl ExchangeTrait for Binance {
//     fn api_base(&self) -> &str {
//         BINANCE_API_BASE
//     }

//     fn client(&self) -> &reqwest::Client {
//         &self.client
//     }

//     fn exchange_name(&self) -> &str {
//         "Binance"
//     }

//     async fn health_check(&self) -> Result<(), MarketScannerError> {
//         // Binance ping endpoint - test connectivity to the REST API
//         let endpoint = "ping";
//         self.get::<serde_json::Value>(endpoint)
//             .await
//             .map_err(|_| MarketScannerError::HealthCheckFailed)?;

//         Ok(())
//     }
// }

// #[async_trait]
// impl CEXTrait for Binance {
//     async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
//         // Validate symbol is not empty
//         if symbol.is_empty() {
//             return Err(MarketScannerError::InvalidSymbol(
//                 "Symbol cannot be empty".to_string(),
//             ));
//         }

//         // Format symbol for Binance
//         let binance_symbol = format_symbol_for_exchange(symbol, &CexExchange::Binance)?;
//         let endpoint = format!("ticker/bookTicker?symbol={}", binance_symbol);

//         let ticker: BinanceBookTickerResponse = self.get(&endpoint).await?;

//         let bid = parse_f64(&ticker.bid_price, "bid price")?;
//         let ask = parse_f64(&ticker.ask_price, "ask price")?;
//         let bid_qty = parse_f64(&ticker.bid_qty, "bid quantity")?;
//         let ask_qty = parse_f64(&ticker.ask_qty, "ask quantity")?;
//         let mid_price = find_mid_price(bid, ask);

//         // Normalize symbol to standard format
//         let standard_symbol = normalize_symbol(&ticker.symbol);

//         Ok(CexPrice {
//             symbol: standard_symbol,
//             mid_price,
//             bid_price: bid,
//             ask_price: ask,
//             bid_qty,
//             ask_qty,
//             timestamp: get_timestamp_millis(),
//             exchange: Exchange::Cex(CexExchange::Binance),
//         })
//     }
// }
