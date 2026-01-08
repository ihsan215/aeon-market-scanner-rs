use crate::common::{CexPrice, MarketScannerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Common exchange enum definition

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Exchange {
    Cex(CexExchange),
    Dex(DexAggregator),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CexExchange {
    Binance,
    Bybit,
    MEXC,
    OKX,
    Gateio,
    Kucoin,
    Bitget,
    Btcturk,
    Htx,
    Coinbase,
    Kraken,
    Bitfinex,
    Upbit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DexAggregator {
    KyberSwap,
}

// Common exchange trait definition
#[async_trait]
pub trait ExchangeTrait: Send + Sync {
    // Exchange specific methods
    fn api_base(&self) -> &str;
    fn client(&self) -> &reqwest::Client;
    fn exchange_name(&self) -> &str;

    // Default implementations
    async fn get<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
    ) -> Result<T, MarketScannerError> {
        let url = format!("{}/{}", self.api_base(), endpoint);
        let response = self.client().get(&url).send().await?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(MarketScannerError::ApiError(format!(
                "{} API error: {} - {}",
                self.exchange_name(),
                status,
                error_text
            )));
        }

        Ok(response.json().await?)
    }

    // Trait methods
    async fn health_check(&self) -> Result<(), MarketScannerError>;
    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError>;
}

// CEX MACRO EXPORTS
#[macro_export]
macro_rules! create_exchange {
    (
        $struct_name:ident
    ) => {
        pub struct $struct_name {
            client: reqwest::Client,
        }

        impl $struct_name {
            pub fn new() -> Self {
                Self {
                    client: $crate::common::create_http_client(),
                }
            }
        }
    };
}
