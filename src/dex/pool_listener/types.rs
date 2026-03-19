//! Pool listener types: pool kind, config, and price updates.

use crate::dex::chains::{ChainId, Token};
use serde::{Deserialize, Serialize};

/// Pool protocol/version (Uniswap vs Pancake, V2/V3/V4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolKind {
    /// Uniswap V2-style pool (pair contract).
    V2Uniswap,
    /// PancakeSwap V2-style pool (pair contract).
    V2Pancake,
    /// Uniswap V3-style pool.
    V3Uniswap,
    /// PancakeSwap V3-style pool.
    V3Pancake,
    /// Uniswap V4-style pool (PoolManager + pool_id).
    V4Uniswap,
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
    /// Fee in basis points (e.g. 30 for V2, 500/3000/10000 for V3). Used for bid/ask spread.
    pub fee_bps: u32,
}

/// Configuration for the pool listener.
#[derive(Debug, Clone)]
pub struct PoolListenerConfig {
    /// WebSocket RPC URL (e.g. `wss://eth-mainnet.g.alchemy.com/v2/...`).
    pub rpc_ws_url: String,
    /// Chain ID (e.g. 1 for Ethereum mainnet).
    pub chain_id: ChainId,
    /// Pool (address, kind, tokens and price direction).
    pub pool: PoolWithTokens,
    /// On WS disconnect/error: 0 = no reconnect; n = up to n reconnects.
    pub reconnect_attempts: u32,
    /// Milliseconds to wait before each reconnect attempt.
    pub reconnect_delay_ms: u64,
}

/// A single DEX price update from the pool (emitted on each Swap or Sync event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPrice {
    pub chain_id: ChainId,
    pub pool_address: String,
    pub pool_kind: PoolKind,
    /// Bid price (token1 per token0, or inverted per direction).
    pub bid: f64,
    /// Ask price (token1 per token0, or inverted per direction).
    pub ask: f64,
    /// Mid price: (bid + ask) / 2.0.
    pub mid: f64,
    pub direction: PriceDirection,
    /// V3/V4: sqrtPriceX96 from swap log. V2: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sqrt_price_x96: Option<u128>,
    pub block_number: u64,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// V4: pool id (bytes32). V2/V3: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pool_id: Option<[u8; 32]>,
}
