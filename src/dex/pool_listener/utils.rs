//! Pool listener utilities: swap event topics and swap log parsing.

use crate::common::MarketScannerError;
use ethers::core::types::{Bytes, H256, U256};
use ethers::core::utils::keccak256;
use std::str::FromStr;

use super::{PoolKind, PriceDirection};

/// Uniswap/Pancake V2 Sync(uint112 reserve0, uint112 reserve1) — same signature on both.
const TOPIC_V2_SYNC: &str = "0x1c411e9a96e071241c2f21f7726b17ae89e3cab4c78be50e062b03a9fffbbad1";

/// Uniswap V3 Swap(address,address,int256,int256,uint160,uint128,int24)
const TOPIC_V3_UNISWAP_SWAP: &str =
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

/// PancakeSwap V3 Swap(address,address,int256,int256,uint160,uint128,int24,uint128,uint128)
const TOPIC_V3_PANCAKE_SWAP: &str =
    "0x19b47279256b2a23a1665c810c8d55a1758940ee09377d4f8d26497a3577dc83";

/// Uniswap V4 Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)
fn v4_uniswap_swap_topic() -> H256 {
    H256::from(keccak256(
        "Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)".as_bytes(),
    ))
}

/// PancakeSwap Infinity ICLPoolManager event: Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24,uint16)
fn v4_pancake_swap_topic() -> H256 {
    H256::from(keccak256(
        "Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24,uint16)".as_bytes(),
    ))
}

/// Returns (topic0, topic1) for the given pool kind. V2 uses Sync event; V3/V4 use protocol-specific Swap. topic1 is Some for V4/V4Infinity (pool_id).
pub(super) fn swap_topic_h256(
    pool_kind: PoolKind,
    pool_id: Option<&[u8; 32]>,
) -> Result<(H256, Option<H256>), MarketScannerError> {
    match pool_kind {
        PoolKind::V2Uniswap | PoolKind::V2Pancake => Ok((
            H256::from_str(TOPIC_V2_SYNC)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?,
            None,
        )),
        PoolKind::V3Uniswap => Ok((
            H256::from_str(TOPIC_V3_UNISWAP_SWAP)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?,
            None,
        )),
        PoolKind::V3Pancake => Ok((
            H256::from_str(TOPIC_V3_PANCAKE_SWAP)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?,
            None,
        )),
        PoolKind::V4Uniswap => {
            let id = pool_id
                .ok_or_else(|| MarketScannerError::WsRpcError("V4 requires pool_id".into()))?;
            Ok((v4_uniswap_swap_topic(), Some(H256::from(*id))))
        }
        PoolKind::V4Infinity => {
            let id = pool_id.ok_or_else(|| {
                MarketScannerError::WsRpcError("V4Pancake requires pool_id".into())
            })?;
            Ok((v4_pancake_swap_topic(), Some(H256::from(*id))))
        }
    }
}

/// V2 Sync data: (uint112 reserve0, uint112 reserve1) — each 32 bytes in ABI encoding.
/// Returns (bid, ask, reserve0_f64, reserve1_f64). Uses PancakeLibrary getAmountOut for 1 unit: fee_bps parametrik (default 30 if 0).
/// amountInWithFee = 1 * (10000 - fee_bps), amountOut = numerator/denominator.
pub(super) fn parse_v2_sync_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
    fee_bps: u32,
) -> Result<(f64, f64, f64, f64), MarketScannerError> {
    if data.len() < 64 {
        return Err(MarketScannerError::WsRpcError(
            "V2 sync data too short".into(),
        ));
    }
    let reserve0 = U256::from_big_endian(&data[0..32]);
    let reserve1 = U256::from_big_endian(&data[32..64]);
    let r0 = reserve0.low_u128() as f64 / 10f64.powi(decimals0 as i32);
    let r1 = reserve1.low_u128() as f64 / 10f64.powi(decimals1 as i32);
    if r0 == 0.0 {
        return Err(MarketScannerError::WsRpcError(
            "V2 sync: reserve0 is zero".into(),
        ));
    }
    let fee_bps = if fee_bps == 0 { 30 } else { fee_bps };
    let fee_bps = fee_bps.min(9999);
    let mul_in_with_fee = (10000 - fee_bps) as f64;
    const DENOM: f64 = 10000.0;

    // Bid: sell 1 token0 → token1. amountOut = (amountInWithFee * reserveOut) / (reserveIn * 10000 + amountInWithFee)
    let bid = (mul_in_with_fee * r1) / (r0 * DENOM + mul_in_with_fee);

    // Ask: get 1 token0 by paying token1. amountIn such that amountOut=1 → amountIn = (reserveIn * 10000) / (mul * (reserveOut - 1))
    let ask = if r0 > 1.0 {
        (r1 * DENOM) / (mul_in_with_fee * (r0 - 1.0))
    } else {
        (r1 / r0) * DENOM / mul_in_with_fee
    };

    Ok((bid, ask, r0, r1))
}

/// V2 Swap data: amount0In, amount1In, amount0Out, amount1Out (each 32 bytes).
/// Returns (price token1/token0, amount0_in_human, amount1_in_human, amount0_out_human, amount1_out_human).
#[allow(dead_code)]
pub(super) fn parse_v2_swap_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, f64, f64, f64, f64), MarketScannerError> {
    if data.len() < 128 {
        return Err(MarketScannerError::WsRpcError(
            "V2 swap data too short".into(),
        ));
    }
    let amount0_in = U256::from_big_endian(&data[0..32]);
    let amount1_in = U256::from_big_endian(&data[32..64]);
    let amount0_out = U256::from_big_endian(&data[64..96]);
    let amount1_out = U256::from_big_endian(&data[96..128]);

    let a0_in = amount0_in.as_u128() as f64 / 10f64.powi(decimals0 as i32);
    let a1_in = amount1_in.as_u128() as f64 / 10f64.powi(decimals1 as i32);
    let a0_out = amount0_out.as_u128() as f64 / 10f64.powi(decimals0 as i32);
    let a1_out = amount1_out.as_u128() as f64 / 10f64.powi(decimals1 as i32);

    // token1 per token0: either (amount1Out/amount0In) or (amount1In/amount0Out)
    let price = if amount0_in > U256::zero() && amount1_out > U256::zero() {
        a1_out / a0_in
    } else if amount1_in > U256::zero() && amount0_out > U256::zero() {
        a1_in / a0_out
    } else {
        return Err(MarketScannerError::WsRpcError(
            "V2 swap: no valid amount pair".into(),
        ));
    };
    Ok((price, a0_in, a1_in, a0_out, a1_out))
}

// --- V3 getAmountOut-style math (Q96, fee 1e6) ---
const Q96: u128 = 1u128 << 96;
const FEE_DENOM_1E6: u64 = 1_000_000;

/// V3 getAmountOut: zero_for_one => sell token0 get token1; !zero_for_one => sell token1 get token0.
/// fee_bps is in 1e6 basis (500 / 3000 / 10000). Uses U256 for overflow safety.
pub(super) fn calc_amount_out(
    sqrt_price_x96: u128,
    liquidity: u128,
    amount_in: u128,
    fee_bps: u32,
    zero_for_one: bool,
) -> Option<u128> {
    let fee_1e6 = (fee_bps as u64).min(999_999);
    let amt_fee =
        (U256::from(amount_in) * U256::from(FEE_DENOM_1E6 - fee_1e6)) / U256::from(FEE_DENOM_1E6);
    if amt_fee.is_zero() {
        return Some(0);
    }
    let q96 = U256::from(Q96);
    let l = U256::from(liquidity);
    let sqrt = U256::from(sqrt_price_x96);

    let out = if zero_for_one {
        // token0 → token1 (bid): bu branch doğru, dokunmuyoruz
        let denom = l + (amt_fee * sqrt / q96);
        if denom.is_zero() {
            return None;
        }
        let new_sqrt = (l * sqrt) / denom;
        if new_sqrt >= sqrt {
            return None;
        }
        let delta_sqrt = sqrt - new_sqrt;
        (l * delta_sqrt) / q96
    } else {
        let amt_q96 = amt_fee * q96;
        let new_sqrt = sqrt + amt_q96 / l;
        if new_sqrt.is_zero() || sqrt.is_zero() {
            return None;
        }

        (amt_fee * q96 * q96) / (new_sqrt * sqrt)
    };
    out.try_into().ok()
}

/// V3 Swap data (ABI, 160 bytes): amount0, amount1, sqrtPriceX96 (slot2), liquidity (slot3), tick.
/// Returns (bid, ask, sqrt_price_x96). bid = token1 per 1 token0; ask = token1 per 1 token0.
pub(super) fn parse_v3_swap_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
    fee_bps: u32,
) -> Result<(f64, f64, u128), MarketScannerError> {
    if data.len() < 160 {
        return Err(MarketScannerError::WsRpcError(
            "V3 swap data too short (need 160)".into(),
        ));
    }
    let sqrt_price_x96 = U256::from_big_endian(&data[64..96]).low_u128();
    let liquidity = U256::from_big_endian(&data[96..128]).low_u128();

    let one0 = 10u128.pow(decimals0 as u32);
    let one1 = 10u128.pow(decimals1 as u32);

    let bid_raw = calc_amount_out(sqrt_price_x96, liquidity, one0, fee_bps, true)
        .ok_or_else(|| MarketScannerError::WsRpcError("bid calc failed".into()))?;
    let ask_raw = calc_amount_out(sqrt_price_x96, liquidity, one1, fee_bps, false)
        .ok_or_else(|| MarketScannerError::WsRpcError("ask calc failed".into()))?;
    if ask_raw == 0 {
        return Err(MarketScannerError::WsRpcError("ask_raw zero".into()));
    }

    // bid: 1 token0 -> token1 = bid_raw/one1
    let bid = bid_raw as f64 / one1 as f64;
    // ask: one1 token1 -> token0 = one0/ask_raw
    let ask = one0 as f64 / ask_raw as f64;

    Ok((bid, ask, sqrt_price_x96))
}

/// V4 Swap data (non-indexed): amount0 (int128), amount1 (int128), sqrtPriceX96 (uint160), liquidity (uint128),
/// tick (int24), fee (uint24) — each 32 bytes = 192 bytes.
/// Returns (bid, ask, sqrt_price_x96). Fee is taken from the swap event (slot5, 1e6 basis).
pub(super) fn parse_v4_swap_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, f64, u128), MarketScannerError> {
    if data.len() < 192 {
        return Err(MarketScannerError::WsRpcError(
            "V4 swap data too short (need 192)".into(),
        ));
    }

    // slot2: sqrtPriceX96, slot3: liquidity, slot5: fee (uint24, encoded in 32 bytes)
    let sqrt_price_x96 = U256::from_big_endian(&data[64..96]).low_u128();
    let liquidity = U256::from_big_endian(&data[96..128]).low_u128();
    let fee_1e6 = U256::from_big_endian(&data[160..192]).low_u32();

    if liquidity == 0 {
        return Err(MarketScannerError::WsRpcError(
            "V4 swap: zero liquidity".into(),
        ));
    }

    let one0 = 10u128.pow(decimals0 as u32);
    let one1 = 10u128.pow(decimals1 as u32);

    let bid_raw = calc_amount_out(sqrt_price_x96, liquidity, one0, fee_1e6, true)
        .ok_or_else(|| MarketScannerError::WsRpcError("V4 bid calc failed".into()))?;
    let ask_raw = calc_amount_out(sqrt_price_x96, liquidity, one1, fee_1e6, false)
        .ok_or_else(|| MarketScannerError::WsRpcError("V4 ask calc failed".into()))?;
    if ask_raw == 0 {
        return Err(MarketScannerError::WsRpcError("V4 ask_raw zero".into()));
    }

    let bid = bid_raw as f64 / one1 as f64;
    let ask = one0 as f64 / ask_raw as f64;

    Ok((bid, ask, sqrt_price_x96))
}

/// Apply direction to bid/ask/mid (token1 per token0). When Reversed: new_bid = 1/ask, new_ask = 1/bid.
pub(super) fn apply_direction_bid_ask(
    bid: f64,
    ask: f64,
    mid: f64,
    direction: PriceDirection,
) -> (f64, f64, f64) {
    match direction {
        PriceDirection::Token1PerToken0 => (bid, ask, mid),
        PriceDirection::Token0PerToken1 => {
            let new_bid = if ask == 0.0 { 0.0 } else { 1.0 / ask };
            let new_ask = if bid == 0.0 { 0.0 } else { 1.0 / bid };
            let new_mid = (new_bid + new_ask) / 2.0;
            (new_bid, new_ask, new_mid)
        }
    }
}
