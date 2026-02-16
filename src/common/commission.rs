//! CEX taker commission rates and effective price helpers.
//!
//! Arbitrage profit uses these effective prices so commission is already deducted.

use std::collections::HashMap;

use crate::common::exchange::{CexExchange, DexAggregator, Exchange};

/// Optional fee overrides for users who want to provide their own tiered/VIP rates.
///
/// Values are decimals (e.g. `0.001` = `0.1%`).
#[derive(Debug, Clone, Default)]
pub struct FeeOverrides {
    pub cex_taker: HashMap<CexExchange, f64>,
    pub dex_taker: HashMap<DexAggregator, f64>,
}

impl FeeOverrides {
    pub fn with_cex_taker_fee(mut self, exchange: CexExchange, fee: f64) -> Self {
        self.cex_taker.insert(exchange, fee);
        self
    }

    pub fn with_dex_taker_fee(mut self, aggregator: DexAggregator, fee: f64) -> Self {
        self.dex_taker.insert(aggregator, fee);
        self
    }
}

/// Taker fee rate (decimal). E.g. 0.001 = 0.1%.
/// Spot trading, default tier. VIP / volume discounts not applied.
pub fn taker_fee_rate(cex: &CexExchange) -> f64 {
    match cex {
        CexExchange::Binance => 0.001,    // 0.10%
        CexExchange::Bybit => 0.001,      // 0.10%
        CexExchange::MEXC => 0.0005,      // 0.05%
        CexExchange::OKX => 0.001,        // 0.10%
        CexExchange::Gateio => 0.001,     // 0.10%
        CexExchange::Kucoin => 0.001,     // 0.10%
        CexExchange::Bitget => 0.001,     // 0.10%
        CexExchange::Btcturk => 0.0012,   // 0.12% base tier
        CexExchange::Htx => 0.002,        // 0.20%
        CexExchange::Coinbase => 0.005,   // 0.50% (between adv/simple)
        CexExchange::Kraken => 0.0026,    // 0.26%
        CexExchange::Bitfinex => 0.002,   // 0.20%
        CexExchange::Upbit => 0.0025,     // 0.25%
        CexExchange::Cryptocom => 0.0004, // 0.04%
    }
}

/// DEX fee rate (decimal). KyberSwap Swap has no platform fee.
fn dex_taker_fee_rate(_dex: &DexAggregator) -> f64 {
    match _dex {
        DexAggregator::KyberSwap => 0.0,
    }
}

/// Taker fee rate (decimal) with optional overrides.
pub fn taker_fee_rate_with_overrides(cex: &CexExchange, overrides: Option<&FeeOverrides>) -> f64 {
    if let Some(ovr) = overrides {
        if let Some(v) = ovr.cex_taker.get(cex) {
            return *v;
        }
    }
    taker_fee_rate(cex)
}

/// DEX fee rate (decimal) with optional overrides.
fn dex_taker_fee_rate_with_overrides(dex: &DexAggregator, overrides: Option<&FeeOverrides>) -> f64 {
    if let Some(ovr) = overrides {
        if let Some(v) = ovr.dex_taker.get(dex) {
            return *v;
        }
    }
    dex_taker_fee_rate(dex)
}

/// Fee rate for any exchange (CEX or DEX). Decimal, e.g. 0.001 = 0.1%.
pub fn fee_rate(exchange: &Exchange) -> f64 {
    match exchange {
        Exchange::Cex(cex) => taker_fee_rate(cex),
        Exchange::Dex(dex) => dex_taker_fee_rate(dex),
    }
}

/// Fee rate for any exchange (CEX or DEX), with optional overrides.
pub fn fee_rate_with_overrides(exchange: &Exchange, overrides: Option<&FeeOverrides>) -> f64 {
    match exchange {
        Exchange::Cex(cex) => taker_fee_rate_with_overrides(cex, overrides),
        Exchange::Dex(dex) => dex_taker_fee_rate_with_overrides(dex, overrides),
    }
}

/// Side for commission: Buy = pay more (amount × (1 + fee)), Sell = receive less (amount × (1 − fee)).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AmountSide {
    Buy,
    Sell,
}

/// Effective amount after commission. Ask → `AmountSide::Buy`, bid → `AmountSide::Sell`.
/// Use for best-buy / best-sell comparison and profit calc.
pub fn effective_price(amount: f64, exchange: &Exchange, side: AmountSide) -> f64 {
    let fee = fee_rate(exchange);
    match side {
        AmountSide::Buy => amount * (1.0 + fee),
        AmountSide::Sell => amount * (1.0 - fee),
    }
}

/// Effective amount after commission, with optional overrides.
pub fn effective_price_with_overrides(
    amount: f64,
    exchange: &Exchange,
    side: AmountSide,
    overrides: Option<&FeeOverrides>,
) -> f64 {
    let fee = fee_rate_with_overrides(exchange, overrides);
    match side {
        AmountSide::Buy => amount * (1.0 + fee),
        AmountSide::Sell => amount * (1.0 - fee),
    }
}
