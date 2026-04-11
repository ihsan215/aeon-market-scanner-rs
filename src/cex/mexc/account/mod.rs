mod auth;
mod spot;
mod stream;
pub(super) mod types;

use crate::common::MarketScannerError;
use tokio::sync::mpsc;

pub use spot::{
    MexcBatchOrderItem, MexcBatchOrderResult, MexcCancelledOrder, MexcLimitOrderType, MexcPlacedOrder,
};
pub use stream::types::{
    MexcAccountBalanceSnapshot, MexcBalance, MexcOrderStatus, MexcOrderType, MexcSpotOrderDisplay,
    MexcSpotOrderUpdate, MexcTradeSide,
};

impl super::Mexc {
    pub async fn get_account_balances(
        &self,
    ) -> Result<MexcAccountBalanceSnapshot, MarketScannerError> {
        stream::get_account_balances(self).await
    }

    pub async fn stream_spot_order_updates(
        &self,
        reconnect_attempts: u32,
        reconnect_delay_ms: u64,
    ) -> Result<mpsc::Receiver<MexcSpotOrderUpdate>, MarketScannerError> {
        stream::stream_spot_order_updates(self, reconnect_attempts, reconnect_delay_ms).await
    }
}
