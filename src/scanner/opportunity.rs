use crate::common::{CexPrice, DexPrice};
use serde::{Deserialize, Serialize};

/// Price data enum - can contain either CEX or DEX price data
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PriceData {
    /// CEX price data
    Cex(CexPrice),
    /// DEX price data
    Dex(DexPrice),
}

/// Arbitrage opportunity: buy from one exchange (source), sell on another (destination).
///
/// Uses standard arbitrage terminology:
/// - **Source leg**: where we acquire the asset (pay effective ask)
/// - **Destination leg**: where we dispose of the asset (receive effective bid)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Source exchange: where we buy (acquire) the asset
    #[serde(alias = "buy_exchange")]
    pub source_exchange: String,
    /// Destination exchange: where we sell (dispose of) the asset
    #[serde(alias = "sell_exchange")]
    pub destination_exchange: String,
    /// Trading pair symbol (e.g. "BTCUSDT")
    pub symbol: String,
    /// Effective cost to acquire (ask × (1 + fee))
    #[serde(alias = "buy_price")]
    pub effective_ask: f64,
    /// Effective proceeds when disposing (bid × (1 − fee))
    #[serde(alias = "sell_price")]
    pub effective_bid: f64,
    /// Arbitrage spread per unit (effective_bid − effective_ask), net of fees
    #[serde(alias = "profit")]
    pub spread: f64,
    /// Spread as percentage ((spread / effective_ask) × 100), net of fees
    #[serde(alias = "profit_percentage")]
    pub spread_percentage: f64,
    /// Maximum executable quantity (min of available depth on both legs)
    #[serde(alias = "buy_quantity", alias = "sell_quantity")]
    pub executable_quantity: f64,
    /// Source leg commission rate in percent (e.g. 0.1 = 0.1%)
    pub source_commission_percent: f64,
    /// Destination leg commission rate in percent (e.g. 0.1 = 0.1%)
    pub destination_commission_percent: f64,
    /// Total commission in quote currency for executable_quantity
    pub total_commission_quote: f64,
    /// Full price data for the source leg (acquire side)
    #[serde(alias = "buy_price_data")]
    pub source_leg: PriceData,
    /// Full price data for the destination leg (dispose side)
    #[serde(alias = "sell_price_data")]
    pub destination_leg: PriceData,
}

impl ArbitrageOpportunity {
    /// Total profit in quote currency (spread × executable quantity)
    pub fn total_profit(&self) -> f64 {
        self.spread * self.executable_quantity
    }
}
