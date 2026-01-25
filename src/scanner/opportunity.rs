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

/// Arbitrage opportunity - opportunity to buy from one exchange and sell on another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageOpportunity {
    /// Exchange name to buy from
    pub buy_exchange: String,
    /// Exchange name to sell on
    pub sell_exchange: String,
    /// Symbol (e.g., "BTCUSDT")
    pub symbol: String,
    /// Buy price (ask price)
    pub buy_price: f64,
    /// Sell price (bid price)
    pub sell_price: f64,
    /// Absolute profit (sell_price - buy_price)
    pub profit: f64,
    /// Profit percentage ((profit / buy_price) * 100)
    pub profit_percentage: f64,
    /// Buy quantity (min(ask_qty, bid_qty))
    pub buy_quantity: f64,
    /// Sell quantity (min(ask_qty, bid_qty))
    pub sell_quantity: f64,
    /// Full price response data for buy side from get_price call
    /// Contains: timestamp, bid_price, ask_price, quantities, exchange info
    /// For DEX: also includes bid_route_summary, ask_route_summary, bid_route_data, ask_route_data
    pub buy_price_data: PriceData,
    /// Full price response data for sell side from get_price call
    /// Contains: timestamp, bid_price, ask_price, quantities, exchange info
    /// For DEX: also includes bid_route_summary, ask_route_summary, bid_route_data, ask_route_data
    pub sell_price_data: PriceData,
}

impl ArbitrageOpportunity {
    /// Calculates total profit (profit * quantity)
    pub fn total_profit(&self) -> f64 {
        let quantity = self.buy_quantity.min(self.sell_quantity);
        self.profit * quantity
    }
}
