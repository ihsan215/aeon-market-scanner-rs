use crate::common::{CexPrice, DexPrice, MarketScannerError};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// Common exchange enum definition

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Exchange {
    Cex(CexExchange),
    Dex(DexAggregator),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
    Cryptocom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
}

// Common Cex Traits
#[async_trait]
pub trait CEXTrait: ExchangeTrait {
    /// Whether this CEX supports fetching price via WebSocket (same format as [get_price]).
    fn supports_websocket(&self) -> bool;

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError>;

    /// Continuous price feed: connection stays open, CexPrice is sent over the channel.
    /// Subscribes to all given symbols; each update includes the symbol in CexPrice.
    /// When the receiver returns None, the connection has closed.
    /// If `reconnect` is true, the implementation should reconnect with backoff when disconnected.
    /// If `max_attempts` is Some(n), stop retrying after n consecutive failed connection attempts.
    /// Default: returns error if this exchange does not support streaming WebSocket.
    async fn stream_price_websocket(
        &self,
        symbols: &[&str],
        reconnect: bool,
        max_attempts: Option<u32>,
    ) -> Result<tokio::sync::mpsc::Receiver<CexPrice>, MarketScannerError> {
        let _ = symbols;
        let _ = reconnect;
        let _ = max_attempts;
        Err(MarketScannerError::ApiError(format!(
            "{} does not support streaming WebSocket",
            self.exchange_name()
        )))
    }
}

#[async_trait]
pub trait DEXTrait: ExchangeTrait {
    async fn get_price(
        &self,
        base_token: &crate::dex::chains::Token,
        quote_token: &crate::dex::chains::Token,
        quote_amount: f64,
    ) -> Result<DexPrice, MarketScannerError>;
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
