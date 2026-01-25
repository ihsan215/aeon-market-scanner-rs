use crate::common::{CexPrice, DexPrice, Exchange};
use serde::{Deserialize, Serialize};

/// Unified price information from any exchange (CEX or DEX)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceInfo {
    Cex(CexPrice),
    Dex(DexPrice),
}

impl PriceInfo {
    pub fn ask_price(&self) -> f64 {
        match self {
            PriceInfo::Cex(price) => price.ask_price,
            PriceInfo::Dex(price) => price.ask_price,
        }
    }

    pub fn bid_price(&self) -> f64 {
        match self {
            PriceInfo::Cex(price) => price.bid_price,
            PriceInfo::Dex(price) => price.bid_price,
        }
    }

    pub fn exchange(&self) -> &Exchange {
        match self {
            PriceInfo::Cex(price) => &price.exchange,
            PriceInfo::Dex(price) => &price.exchange,
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            PriceInfo::Cex(price) => &price.symbol,
            PriceInfo::Dex(price) => &price.symbol,
        }
    }

    pub fn ask_qty(&self) -> f64 {
        match self {
            PriceInfo::Cex(price) => price.ask_qty,
            PriceInfo::Dex(price) => price.ask_qty,
        }
    }

    pub fn bid_qty(&self) -> f64 {
        match self {
            PriceInfo::Cex(price) => price.bid_qty,
            PriceInfo::Dex(price) => price.bid_qty,
        }
    }
}

/// Represents the best exchange for buying (lowest ask price)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestBuy {
    pub exchange: Exchange,
    pub price: f64,
    pub quantity: f64,
    pub price_info: PriceInfo,
}

/// Represents the best exchange for selling (highest bid price)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BestSell {
    pub exchange: Exchange,
    pub price: f64,
    pub quantity: f64,
    pub price_info: PriceInfo,
}

/// Market scan result showing arbitrage opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketScanResult {
    pub symbol: String,
    pub best_buy: BestBuy,
    pub best_sell: BestSell,
    /// Potential profit percentage: ((sell_price - buy_price) / buy_price) * 100
    pub profit_percentage: f64,
    /// Maximum tradeable quantity (min of buy and sell quantities)
    pub max_tradeable_qty: f64,
    /// Potential profit in quote currency for max_tradeable_qty
    pub potential_profit: f64,
    pub timestamp: u64,
}

impl MarketScanResult {
    /// Calculate if there's a profitable arbitrage opportunity
    /// Returns true if sell_price > buy_price (after considering fees/spread)
    pub fn is_profitable(&self, min_profit_percentage: f64) -> bool {
        self.profit_percentage >= min_profit_percentage
    }
}
