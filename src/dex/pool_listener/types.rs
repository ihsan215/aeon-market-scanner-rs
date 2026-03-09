//! Pool listener types: pool kind, config, and price updates.

use crate::dex::chains::Token;
use serde::{Deserialize, Serialize};

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

/// A single DEX price update from the pool (emitted on each Swap event).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexPrice {
    pub chain_id: u64,
    pub pool_address: String,
    pub pool_kind: PoolKind,
    /// Price derived from swap event parameters.
    pub price: f64,
    pub direction: PriceDirection,
    /// V3: sqrtPriceX96 from swap log. V2: None.
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
