//! DEX pool price listener over WebSocket RPC (ethers-rs).
//!
//! Connects to an Ethereum node via WebSocket, subscribes to Swap events (eth_subscribe "logs"),
//! and emits price updates computed from swap event parameters. Uniswap V2, V3 or V4 style pools.

use crate::common::{MarketScannerError, get_timestamp_millis};
use crate::dex::chains::Token;
use ethers::core::types::{Address, Bytes, Filter, H256, I256, U256};
use ethers::core::utils::keccak256;
use ethers::providers::{Middleware, Provider, Ws};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio::time::Duration;

/// Uniswap V2, V3, V4 or PancakeSwap Infinity (V4-style) pool type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolKind {
    V2,
    V3,
    V4,
    /// PancakeSwap Infinity CLPoolManager; Swap(bytes32,address,int256,int256,uint160,uint128,int24,uint24,uint24)
    V4Infinity,
}

/// Price quote direction: which unit the price is expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceDirection {
    /// Price = token1 per token0 (e.g. USDT per BNB).
    Token1PerToken0,
    /// Price = token0 per token1.
    Token0PerToken1,
}

/// Pool address, tokens, kind and price direction (decimals and symbol are taken from the tokens).
#[derive(Debug, Clone)]
pub struct PoolWithTokens {
    /// Pool contract address (V2 pair, V3 pool). For V4: PoolManager contract address.
    pub pool_address: String,
    /// V2, V3 or V4 pool type.
    pub pool_kind: PoolKind,
    /// For V4 only: pool id (bytes32) to filter Swap events by pool. Required when pool_kind is V4.
    pub pool_id: Option<[u8; 32]>,
    /// Token0 (base); decimals and symbol are read from here.
    pub token0: Token,
    /// Token1 (quote); decimals and symbol are read from here.
    pub token1: Token,
    /// Price quote direction: token1/token0 or token0/token1.
    pub price_direction: PriceDirection,
}

/// Configuration for the pool listener.
#[derive(Debug, Clone)]
pub struct PoolListenerConfig {
    /// WebSocket RPC URL (e.g. `wss://eth-mainnet.g.alchemy.com/v2/...`).
    pub rpc_ws_url: String,
    /// Chain ID (e.g. 1 for Ethereum mainnet).
    pub chain_id: u64,
    /// Pool (address, kind, tokens and price direction).
    pub pool: PoolWithTokens,
    /// On WS disconnect/error: 0 = no reconnect; n = up to n reconnects.
    pub reconnect_attempts: u32,
    /// Milliseconds to wait before each reconnect attempt.
    pub reconnect_delay_ms: u64,
}

/// A single price update from the pool (emitted on each Swap event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceUpdate {
    pub chain_id: u64,
    pub pool_address: String,
    pub pool_kind: PoolKind,
    /// Price derived from swap event parameters.
    pub price: f64,
    pub direction: PriceDirection,
    /// V2: from swap amounts. V3: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve0: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve1: Option<f64>,
    /// V3: sqrtPriceX96 from swap log. V2: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sqrt_price_x96: Option<u128>,
    pub block_number: u64,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

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

fn swap_topic_h256(
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

/// Loads `.env` from the current or project directory. Call before reading env vars (e.g. in tests).
pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

/// Subscribe to pool swap events over WebSocket (eth_subscribe "logs" only).
/// Price is computed from swap event parameters; no block subscription or public RPC.
pub async fn stream_pool_prices(
    config: PoolListenerConfig,
) -> Result<mpsc::Receiver<PoolPriceUpdate>, MarketScannerError> {
    let (tx, rx) = mpsc::channel(64);
    let rpc_ws_url = config.rpc_ws_url.clone();
    let chain_id = config.chain_id;
    let pool = config.pool.clone();
    let reconnect_attempts = config.reconnect_attempts;
    let reconnect_delay_ms = config.reconnect_delay_ms;

    tokio::spawn(async move {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match run_listener(rpc_ws_url.clone(), chain_id, pool.clone(), tx.clone()).await {
                Ok(()) => {
                    eprintln!("[pool_listener] connection closed (stream ended)");
                }
                Err(e) => {
                    eprintln!("[pool_listener] run_listener error: {}", e);
                }
            }
            if reconnect_attempts == 0 || attempt > reconnect_attempts {
                eprintln!(
                    "[pool_listener] not reconnecting (runs={}, max_reconnects={})",
                    attempt, reconnect_attempts
                );
                break;
            }
            let delay = Duration::from_millis(reconnect_delay_ms);
            eprintln!(
                "[pool_listener] reconnecting in {:?} (run {} done, up to {} reconnects)",
                delay, attempt, reconnect_attempts
            );
            tokio::time::sleep(delay).await;
        }
    });

    Ok(rx)
}

async fn run_listener(
    rpc_ws_url: String,
    chain_id: u64,
    pool: PoolWithTokens,
    tx: mpsc::Sender<PoolPriceUpdate>,
) -> Result<(), MarketScannerError> {
    let ws_provider = Provider::<Ws>::connect(&rpc_ws_url)
        .await
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    let pool_address = pool.pool_address.clone();
    let pool_addr = Address::from_str(pool_address.trim_start_matches("0x"))
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    let decimals0 = pool.token0.decimal;
    let decimals1 = pool.token1.decimal;
    let symbol = format!("{}{}", pool.token0.symbol, pool.token1.symbol);
    let price_direction = pool.price_direction;
    let pool_kind = pool.pool_kind;

    let (topic0, topic1) = swap_topic_h256(pool_kind, pool.pool_id.as_ref())?;
    let filter = match topic1 {
        None => Filter::new().address(pool_addr).topic0(topic0),
        Some(t1) => Filter::new().address(pool_addr).topic0(topic0).topic1(t1),
    };

    let mut log_stream = ws_provider
        .subscribe_logs(&filter)
        .await
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    while let Some(log) = log_stream.next().await {
        // Print incoming swap data
        eprintln!(
            "[pool_listener] swap log: block={:?} data_len={}",
            log.block_number,
            log.data.len()
        );

        match pool_kind {
            PoolKind::V2 => {
                if let Ok((price, amount0_in, amount1_in, amount0_out, amount1_out)) =
                    parse_v2_swap_and_price(&log.data, decimals0, decimals1)
                {
                    eprintln!(
                        "[pool_listener] V2 swap: amount0In={} amount1In={} amount0Out={} amount1Out={} => price(token1/token0)={}",
                        amount0_in, amount1_in, amount0_out, amount1_out, price
                    );
                    let block_number = log.block_number.unwrap_or_default().as_u64();
                    let price = apply_direction(price, price_direction);
                    let update = PoolPriceUpdate {
                        chain_id,
                        pool_address: pool_address.clone(),
                        pool_kind,
                        price,
                        direction: price_direction,
                        reserve0: None,
                        reserve1: None,
                        sqrt_price_x96: None,
                        block_number,
                        timestamp: get_timestamp_millis(),
                        symbol: Some(symbol.clone()),
                    };
                    if tx.send(update).await.is_err() {
                        break;
                    }
                }
            }
            PoolKind::V3 => {
                if let Ok((price, sqrt_price_x96, amount0, amount1)) =
                    parse_v3_swap_and_price(&log.data, decimals0, decimals1)
                {
                    eprintln!(
                        "[pool_listener] V3 swap: amount0={} amount1={} sqrtPriceX96={} => price(token1/token0)={}",
                        amount0, amount1, sqrt_price_x96, price
                    );
                    let block_number = log.block_number.unwrap_or_default().as_u64();
                    let price = apply_direction(price, price_direction);
                    let update = PoolPriceUpdate {
                        chain_id,
                        pool_address: pool_address.clone(),
                        pool_kind,
                        price,
                        direction: price_direction,
                        reserve0: None,
                        reserve1: None,
                        sqrt_price_x96: Some(sqrt_price_x96),
                        block_number,
                        timestamp: get_timestamp_millis(),
                        symbol: Some(symbol.clone()),
                    };
                    if tx.send(update).await.is_err() {
                        break;
                    }
                }
            }
            PoolKind::V4 | PoolKind::V4Infinity => {
                if let Ok((price, sqrt_price_x96, amount0, amount1)) =
                    parse_v4_swap_and_price(&log.data, decimals0, decimals1)
                {
                    eprintln!(
                        "[pool_listener] V4/V4Infinity swap: amount0={} amount1={} sqrtPriceX96={} => price(token1/token0)={}",
                        amount0, amount1, sqrt_price_x96, price
                    );
                    let block_number = log.block_number.unwrap_or_default().as_u64();
                    let price = apply_direction(price, price_direction);
                    let update = PoolPriceUpdate {
                        chain_id,
                        pool_address: pool_address.clone(),
                        pool_kind,
                        price,
                        direction: price_direction,
                        reserve0: None,
                        reserve1: None,
                        sqrt_price_x96: Some(sqrt_price_x96),
                        block_number,
                        timestamp: get_timestamp_millis(),
                        symbol: Some(symbol.clone()),
                    };
                    if tx.send(update).await.is_err() {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

fn apply_direction(raw_token1_per_token0: f64, direction: PriceDirection) -> f64 {
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

/// V2 Swap data: amount0In, amount1In, amount0Out, amount1Out (each 32 bytes).
/// Returns (price token1/token0, amount0_in_human, amount1_in_human, amount0_out_human, amount1_out_human).
fn parse_v2_swap_and_price(
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
fn parse_v3_swap_and_price(
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
fn parse_v4_swap_and_price(
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
