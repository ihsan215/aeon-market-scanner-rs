//! Pool listener utilities: swap event topics and swap log parsing.

use crate::common::MarketScannerError;
use ethers::core::types::{Bytes, H256, I256, U256};
use ethers::core::utils::keccak256;
use std::str::FromStr;

use super::{PoolKind, PriceDirection};

/// Uniswap V2 Swap(address,uint256,uint256,uint256,uint256,address)
const TOPIC_V2_SWAP: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
/// Uniswap V3 Swap(address,address,int256,int256,uint160,uint128,int24)
const TOPIC_V3_SWAP: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

/// Uniswap V4 Swap(bytes32,address,int128,int128,uint160,uint128,int24,uint24)
fn v4_swap_topic() -> H256 {
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

/// Returns (topic0, topic1) for the given pool kind. topic1 is Some for V4/V4Infinity (pool_id).
pub(super) fn swap_topic_h256(
    pool_kind: PoolKind,
    pool_id: Option<&[u8; 32]>,
) -> Result<(H256, Option<H256>), MarketScannerError> {
    match pool_kind {
        PoolKind::V2 => Ok((
            H256::from_str(TOPIC_V2_SWAP)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?,
            None,
        )),
        PoolKind::V3 => Ok((
            H256::from_str(TOPIC_V3_SWAP)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?,
            None,
        )),
        PoolKind::V4 => {
            let id = pool_id
                .ok_or_else(|| MarketScannerError::WsRpcError("V4 requires pool_id".into()))?;
            Ok((v4_swap_topic(), Some(H256::from(*id))))
        }
        PoolKind::V4Infinity => {
            let id = pool_id.ok_or_else(|| {
                MarketScannerError::WsRpcError("V4Pancake requires pool_id".into())
            })?;
            Ok((v4_pancake_swap_topic(), Some(H256::from(*id))))
        }
    }
}

/// V2 Swap data: amount0In, amount1In, amount0Out, amount1Out (each 32 bytes).
/// Returns (price token1/token0, amount0_in_human, amount1_in_human, amount0_out_human, amount1_out_human).
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

/// V3 Swap data: amount0 (int256), amount1 (int256), sqrtPriceX96 (uint160), liquidity, tick (each 32 bytes).
/// Returns (price token1/token0, sqrt_price_x96 for display, amount0_human, amount1_human).
pub(super) fn parse_v3_swap_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, u128, f64, f64), MarketScannerError> {
    if data.len() < 96 {
        return Err(MarketScannerError::WsRpcError(
            "V3 swap data too short".into(),
        ));
    }
    // amount0, amount1 are int256 (signed); use I256 to avoid overflow when casting negative to u128
    let amount0 = I256::from_raw(U256::from_big_endian(&data[0..32]));
    let amount1 = I256::from_raw(U256::from_big_endian(&data[32..64]));
    let a0_mag = amount0.unsigned_abs();
    let a1_mag = amount1.unsigned_abs();

    // sqrtPriceX96 is uint160; can exceed u128::MAX, so use low_u128() to avoid panic
    let sqrt_raw = U256::from_big_endian(&data[64..96]);
    let sqrt_f = u256_to_f64(sqrt_raw);
    let q96 = 2f64.powi(96);
    let price = (sqrt_f / q96).powi(2);
    let decimals_adj = 10f64.powi((decimals1 as i32) - (decimals0 as i32));
    let price = price * decimals_adj;

    let sqrt_price_x96 = sqrt_raw.low_u128();
    let a0 = a0_mag.low_u128() as f64 / 10f64.powi(decimals0 as i32);
    let a1 = a1_mag.low_u128() as f64 / 10f64.powi(decimals1 as i32);
    Ok((price, sqrt_price_x96, a0, a1))
}

/// V4 Swap data (non-indexed): amount0 (int128), amount1 (int128), sqrtPriceX96 (uint160), liquidity (uint128), tick (int24), fee (uint24) — each 32 bytes = 192 bytes.
/// Returns (price token1/token0, sqrt_price_x96 for display, amount0_human, amount1_human).
pub(super) fn parse_v4_swap_and_price(
    data: &Bytes,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, u128, f64, f64), MarketScannerError> {
    if data.len() < 96 {
        return Err(MarketScannerError::WsRpcError(
            "V4 swap data too short".into(),
        ));
    }
    let amount0 = I256::from_raw(U256::from_big_endian(&data[0..32]));
    let amount1 = I256::from_raw(U256::from_big_endian(&data[32..64]));
    let a0_mag = amount0.unsigned_abs();
    let a1_mag = amount1.unsigned_abs();

    let sqrt_raw = U256::from_big_endian(&data[64..96]);
    let sqrt_f = u256_to_f64(sqrt_raw);
    let q96 = 2f64.powi(96);
    let price = (sqrt_f / q96).powi(2);
    let decimals_adj = 10f64.powi((decimals1 as i32) - (decimals0 as i32));
    let price = price * decimals_adj;

    let sqrt_price_x96 = sqrt_raw.low_u128();
    let a0 = a0_mag.low_u128() as f64 / 10f64.powi(decimals0 as i32);
    let a1 = a1_mag.low_u128() as f64 / 10f64.powi(decimals1 as i32);
    Ok((price, sqrt_price_x96, a0, a1))
}

/// Converts U256 to f64 without panicking (U256::as_u128() overflows for values > u128::MAX).
fn u256_to_f64(v: U256) -> f64 {
    const U128_MAX: u128 = u128::MAX;
    if v <= U256::from(U128_MAX) {
        v.low_u128() as f64
    } else {
        // Scale down so it fits in u128, then scale up in f64
        let bits = 256u32.saturating_sub(v.leading_zeros());
        let shift = bits.saturating_sub(127);
        let shifted = v >> shift;
        shifted.low_u128() as f64 * 2f64.powi(shift as i32)
    }
}

/// Apply price direction: return raw (token1/token0) or its inverse (token0/token1).
pub(super) fn apply_direction(raw_token1_per_token0: f64, direction: PriceDirection) -> f64 {
    match direction {
        PriceDirection::Token1PerToken0 => raw_token1_per_token0,
        PriceDirection::Token0PerToken1 => {
            if raw_token1_per_token0 == 0.0 {
                0.0
            } else {
                1.0 / raw_token1_per_token0
            }
        }
    }
}
