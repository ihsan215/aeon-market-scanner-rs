use crate::common::get_timestamp_millis;
use crate::common::{CEXTrait, CexPrice, DEXTrait, DexPrice, Exchange, MarketScannerError};
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

/// Scan multiple exchanges to find the best buy (lowest ask) and sell (highest bid) prices
///
/// # Arguments
/// * `cex_exchanges` - Vector of CEX exchange instances to scan
/// * `dex_exchanges` - Vector of tuples: (DEX instance, base_token, quote_token)
/// * `symbol` - Trading pair symbol (e.g., "BTCUSDT") for CEX exchanges
/// * `quote_amount` - Amount in quote currency to query for DEX (e.g., 1000.0 for $1000)
///
/// # Returns
/// `MarketScanResult` containing best buy and sell opportunities
pub async fn scan_market(
    cex_exchanges: Vec<Box<dyn CEXTrait>>,
    dex_exchanges: Vec<(
        Box<dyn DEXTrait>,
        Option<&crate::dex::chains::Token>,
        Option<&crate::dex::chains::Token>,
    )>,
    symbol: &str,
    quote_amount: f64,
) -> Result<MarketScanResult, MarketScannerError> {
    use futures::future::join_all;

    // Collect all price queries
    let mut price_futures = Vec::new();

    // Query all CEX exchanges
    for cex in &cex_exchanges {
        let cex_clone = cex;
        let symbol_clone = symbol.to_string();
        price_futures
            .push(async move { cex_clone.get_price(&symbol_clone).await.map(PriceInfo::Cex) });
    }

    // Query all DEX exchanges (only if tokens are provided)
    for (dex, base_token, quote_token) in &dex_exchanges {
        if let (Some(base), Some(quote)) = (base_token, quote_token) {
            let dex_clone = dex;
            let base_clone = base.clone();
            let quote_clone = quote.clone();
            let amount = quote_amount;
            price_futures.push(async move {
                dex_clone
                    .get_price(&base_clone, &quote_clone, amount)
                    .await
                    .map(PriceInfo::Dex)
            });
        }
    }

    // Execute all queries in parallel
    let results: Vec<Result<PriceInfo, MarketScannerError>> = join_all(price_futures).await;

    // Filter successful results
    let mut prices: Vec<PriceInfo> = results.into_iter().filter_map(|r| r.ok()).collect();

    if prices.is_empty() {
        return Err(MarketScannerError::ApiError(
            "No exchange returned valid price data".to_string(),
        ));
    }

    // Find best buy (lowest ask price)
    let best_buy_info = prices
        .iter()
        .min_by(|a, b| {
            a.ask_price()
                .partial_cmp(&b.ask_price())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| MarketScannerError::ApiError("No valid ask prices found".to_string()))?;

    let best_buy = BestBuy {
        exchange: best_buy_info.exchange().clone(),
        price: best_buy_info.ask_price(),
        quantity: best_buy_info.ask_qty(),
        price_info: best_buy_info.clone(),
    };

    // Find best sell (highest bid price)
    let best_sell_info = prices
        .iter()
        .max_by(|a, b| {
            a.bid_price()
                .partial_cmp(&b.bid_price())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .ok_or_else(|| MarketScannerError::ApiError("No valid bid prices found".to_string()))?;

    let best_sell = BestSell {
        exchange: best_sell_info.exchange().clone(),
        price: best_sell_info.bid_price(),
        quantity: best_sell_info.bid_qty(),
        price_info: best_sell_info.clone(),
    };

    // Calculate profit metrics
    let max_tradeable_qty = best_buy.quantity.min(best_sell.quantity);
    let profit_per_unit = best_sell.price - best_buy.price;
    let profit_percentage = if best_buy.price > 0.0 {
        (profit_per_unit / best_buy.price) * 100.0
    } else {
        0.0
    };
    let potential_profit = profit_per_unit * max_tradeable_qty;

    Ok(MarketScanResult {
        symbol: symbol.to_string(),
        best_buy,
        best_sell,
        profit_percentage,
        max_tradeable_qty,
        potential_profit,
        timestamp: get_timestamp_millis(),
    })
}
