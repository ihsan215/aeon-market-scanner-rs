//! Chain-level gas cost estimates (USD).
//!
//! These are coarse heuristics meant for ranking/estimation, not execution.

use crate::dex::chains::ChainId;

/// Estimated per-tx gas cost in USD (very rough).
///
/// Note: you asked for "BNB de 1 cent" — this sets BSC to $0.01. Others are placeholders
/// and should be tuned based on your expected transaction type and typical gas usage.
pub fn default_gass_fee_usd(chain_id: ChainId) -> f64 {
    match chain_id {
        // BSC: ~1 cent (as requested)
        ChainId::BSC => 0.01,

        // TODO: tune these per chain
        ChainId::ETHEREUM => 1.00,
        ChainId::ARBITRUM => 0.05,
        ChainId::OPTIMISM => 0.05,
        ChainId::BASE => 0.05,
        ChainId::POLYGON => 0.01,
        ChainId::AVALANCHE => 0.05,
        ChainId::LINEA => 0.05,
        ChainId::MANTLE => 0.02,
        ChainId::PLASMA => 0.01,
        ChainId::UNICHAIN => 0.05,
        ChainId::SONIC => 0.01,
        ChainId::RONIN => 0.01,
        ChainId::HyperEVM => 0.05,
    }
}
