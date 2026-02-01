//! CEX taker commission rates and effective price helpers.
//!
//! Arbitrage profit uses these effective prices so commission is already deducted.

use crate::common::exchange::{CexExchange, DexAggregator, Exchange};

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

/// Fee rate for any exchange (CEX or DEX). Decimal, e.g. 0.001 = 0.1%.
pub fn fee_rate(exchange: &Exchange) -> f64 {
    match exchange {
        Exchange::Cex(cex) => taker_fee_rate(cex),
        Exchange::Dex(dex) => dex_taker_fee_rate(dex),
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
