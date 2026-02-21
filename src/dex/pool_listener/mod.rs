//! DEX pool price listener over WebSocket RPC (ethers-rs).
//!
//! Connects to an Ethereum node via WebSocket, subscribes to new blocks or Swap events,
//! and emits price updates for Uniswap V2 or V3 style pools.

use crate::common::{MarketScannerError, get_timestamp_millis};
use ethers::core::types::{Address, Bytes, Filter, H256, TransactionRequest, U256};
use ethers::providers::{Middleware, Provider, Ws};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use tokio::sync::mpsc;
use tokio::time::Duration;

/// Uniswap V2 or V3 pool type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolKind {
    V2,
    V3,
}

/// Price quote direction: which unit the price is expressed in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceDirection {
    /// Price = token1 per token0 (e.g. USDT per BNB).
    Token1PerToken0,
    /// Price = token0 per token1.
    Token0PerToken1,
}

/// When to emit a price update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListenMode {
    /// Emit on each new block event from the RPC (re-query reserves/slot0).
    /// Block numbers may not be consecutive: the RPC may skip blocks (throttling, reorgs, or provider behaviour).
    EveryBlock,
    /// Emit only when a Swap event is logged for the pool.
    OnSwapEvent,
}

/// Configuration for the pool listener.
#[derive(Debug, Clone)]
pub struct PoolListenerConfig {
    /// WebSocket RPC URL (e.g. `wss://eth-mainnet.g.alchemy.com/v2/...` or `wss://mainnet.infura.io/ws/v3/...`).
    pub rpc_ws_url: String,
    /// Chain ID (e.g. 1 for Ethereum mainnet).
    pub chain_id: u64,
    /// Pool contract address (V2 pair or V3 pool).
    pub pool_address: String,
    /// V2 or V3 pool.
    pub pool_kind: PoolKind,
    /// When to emit updates.
    pub listen_mode: ListenMode,
    /// How to quote price: token1/token0 or token0/token1.
    pub price_direction: PriceDirection,
    /// Optional symbol for the pair (e.g. "ETHUSDT") for the emitted price.
    pub symbol: Option<String>,
    /// On WS disconnect/error: 0 = no reconnect; n = up to n reconnects (1 initial run + n retries).
    pub reconnect_attempts: u32,
    /// Milliseconds to wait before each reconnect attempt.
    pub reconnect_delay_ms: u64,
}

/// A single price update from the pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolPriceUpdate {
    pub chain_id: u64,
    pub pool_address: String,
    pub pool_kind: PoolKind,
    /// Single price; interpretation depends on [PriceDirection].
    pub price: f64,
    /// How the price is quoted (token1/token0 or token0/token1).
    pub direction: PriceDirection,
    /// V2: reserve of token0 (human-readable). V3: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve0: Option<f64>,
    /// V2: reserve of token1 (human-readable). V3: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reserve1: Option<f64>,
    /// V3: sqrtPriceX96 from slot0. V2: None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sqrt_price_x96: Option<u128>,
    pub block_number: u64,
    pub timestamp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
}

// Selectors (first 4 bytes of keccak256)
const SELECTOR_GET_RESERVES: &[u8] = &[0x09, 0x02, 0xf1, 0xac];
const SELECTOR_SLOT0: &[u8] = &[0x38, 0x50, 0xc7, 0xbd];
const SELECTOR_TOKEN0: &[u8] = &[0x0d, 0xfe, 0x16, 0x81];
const SELECTOR_TOKEN1: &[u8] = &[0xd2, 0x12, 0x20, 0xa7];
const SELECTOR_DECIMALS: &[u8] = &[0x31, 0x3c, 0xe5, 0x67];

/// Uniswap V2 Swap(address,uint256,uint256,uint256,uint256,address)
const TOPIC_V2_SWAP: &str = "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822";
/// Uniswap V3 Swap(address,address,int256,int256,uint160,uint128,int24)
const TOPIC_V3_SWAP: &str = "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67";

fn swap_topic(pool_kind: PoolKind) -> &'static str {
    match pool_kind {
        PoolKind::V2 => TOPIC_V2_SWAP,
        PoolKind::V3 => TOPIC_V3_SWAP,
    }
}

/// Loads `.env` from the current or project directory. Call before reading env vars (e.g. in tests).
pub fn load_dotenv() {
    let _ = dotenvy::dotenv();
}

/// Subscribe to pool price updates over WebSocket RPC (ethers-rs).
/// Returns a receiver of [PoolPriceUpdate]; the stream runs until the connection closes or an error occurs.
pub async fn stream_pool_prices(
    config: PoolListenerConfig,
) -> Result<mpsc::Receiver<PoolPriceUpdate>, MarketScannerError> {
    let (tx, rx) = mpsc::channel(64);
    let pool_address = config.pool_address.clone();
    let rpc_ws_url = config.rpc_ws_url.clone();
    let chain_id = config.chain_id;
    let pool_kind = config.pool_kind;
    let listen_mode = config.listen_mode;
    let price_direction = config.price_direction;
    let symbol = config.symbol.clone();
    let reconnect_attempts = config.reconnect_attempts;
    let reconnect_delay_ms = config.reconnect_delay_ms;

    tokio::spawn(async move {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            match run_listener(
                rpc_ws_url.clone(),
                chain_id,
                pool_address.clone(),
                pool_kind,
                listen_mode,
                price_direction,
                symbol.clone(),
                tx.clone(),
            )
            .await
            {
                Ok(()) => {
                    eprintln!("[pool_listener] connection closed (stream ended)");
                }
                Err(e) => {
                    eprintln!("[pool_listener] run_listener error: {}", e);
                }
            }
            if reconnect_attempts == 0 || attempt > reconnect_attempts {
                eprintln!("[pool_listener] not reconnecting (runs={}, max_reconnects={})", attempt, reconnect_attempts);
                break;
            }
            let delay = Duration::from_millis(reconnect_delay_ms);
            eprintln!("[pool_listener] reconnecting in {:?} (run {} done, up to {} reconnects)", delay, attempt, reconnect_attempts);
            tokio::time::sleep(delay).await;
        }
    });

    Ok(rx)
}

async fn run_listener(
    rpc_ws_url: String,
    chain_id: u64,
    pool_address: String,
    pool_kind: PoolKind,
    listen_mode: ListenMode,
    price_direction: PriceDirection,
    symbol: Option<String>,
    tx: mpsc::Sender<PoolPriceUpdate>,
) -> Result<(), MarketScannerError> {
    let provider = Provider::<Ws>::connect(&rpc_ws_url)
        .await
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    let pool_addr = Address::from_str(pool_address.trim_start_matches("0x"))
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    let (decimals0, decimals1) = fetch_decimals(&provider, &pool_addr).await?;

    match listen_mode {
        ListenMode::EveryBlock => {
            let mut block_stream = provider
                .watch_blocks()
                .await
                .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

            let mut last_emitted_block: Option<u64> = None;

            while let Some(block_hash) = block_stream.next().await {
                let _ = block_hash;
                let block_number = provider
                    .get_block_number()
                    .await
                    .map(|n| n.as_u64())
                    .unwrap_or(0);
                // Only emit once per new block (RPC may send duplicate events for same block).
                if last_emitted_block.map_or(true, |b| block_number > b) {
                    last_emitted_block = Some(block_number);
                    if let Ok(data) =
                        fetch_price(&provider, &pool_addr, pool_kind, decimals0, decimals1).await
                    {
                        let price = apply_direction(data.price, price_direction);
                        let update = PoolPriceUpdate {
                            chain_id,
                            pool_address: pool_address.clone(),
                            pool_kind,
                            price,
                            direction: price_direction,
                            reserve0: data.reserve0,
                            reserve1: data.reserve1,
                            sqrt_price_x96: data.sqrt_price_x96,
                            block_number,
                            timestamp: get_timestamp_millis(),
                            symbol: symbol.clone(),
                        };
                        if tx.send(update).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
        ListenMode::OnSwapEvent => {
            let topic = swap_topic(pool_kind);
            let topic0 = H256::from_str(topic)
                .map_err(|_| MarketScannerError::WsRpcError("invalid topic".into()))?;
            let filter = Filter::new().address(pool_addr).topic0(topic0);

            let mut log_stream = provider
                .watch(&filter)
                .await
                .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

            while let Some(log) = log_stream.next().await {
                if let Ok(data) =
                    fetch_price(&provider, &pool_addr, pool_kind, decimals0, decimals1).await
                {
                    let block_number = log.block_number.unwrap_or_default().as_u64();
                    let price = apply_direction(data.price, price_direction);
                    let update = PoolPriceUpdate {
                        chain_id,
                        pool_address: pool_address.clone(),
                        pool_kind,
                        price,
                        direction: price_direction,
                        reserve0: data.reserve0,
                        reserve1: data.reserve1,
                        sqrt_price_x96: data.sqrt_price_x96,
                        block_number,
                        timestamp: get_timestamp_millis(),
                        symbol: symbol.clone(),
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

/// Internal: raw price is always token1/token0; convert to requested direction.
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

async fn eth_call(
    provider: &Provider<Ws>,
    to: Address,
    data: &[u8],
) -> Result<Bytes, MarketScannerError> {
    let tx = TransactionRequest::new()
        .to(to)
        .data(Bytes::from(data.to_vec()));
    provider
        .call(&tx.into(), None)
        .await
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))
}

async fn fetch_decimals(
    provider: &Provider<Ws>,
    pool: &Address,
) -> Result<(u8, u8), MarketScannerError> {
    let token0 = eth_call(provider, *pool, SELECTOR_TOKEN0).await?;
    let token1 = eth_call(provider, *pool, SELECTOR_TOKEN1).await?;
    let addr0 = bytes_to_address(&token0)?;
    let addr1 = bytes_to_address(&token1)?;
    let dec0 = eth_call(provider, addr0, SELECTOR_DECIMALS).await?;
    let dec1 = eth_call(provider, addr1, SELECTOR_DECIMALS).await?;
    let d0 =
        bytes_to_u8(&dec0).ok_or_else(|| MarketScannerError::WsRpcError("decimals0".into()))?;
    let d1 =
        bytes_to_u8(&dec1).ok_or_else(|| MarketScannerError::WsRpcError("decimals1".into()))?;
    Ok((d0, d1))
}

fn bytes_to_address(b: &Bytes) -> Result<Address, MarketScannerError> {
    if b.len() < 32 {
        return Err(MarketScannerError::WsRpcError(
            "token address too short".into(),
        ));
    }
    let mut arr = [0u8; 20];
    arr.copy_from_slice(&b[b.len() - 20..]);
    Ok(Address::from(arr))
}

fn bytes_to_u8(b: &Bytes) -> Option<u8> {
    if b.len() < 32 {
        return None;
    }
    Some(b[b.len() - 1])
}

struct PriceAndRaw {
    price: f64,
    reserve0: Option<f64>,
    reserve1: Option<f64>,
    sqrt_price_x96: Option<u128>,
}

async fn fetch_price(
    provider: &Provider<Ws>,
    pool: &Address,
    pool_kind: PoolKind,
    decimals0: u8,
    decimals1: u8,
) -> Result<PriceAndRaw, MarketScannerError> {
    match pool_kind {
        PoolKind::V2 => {
            let (price, r0, r1) = fetch_v2_price(provider, pool, decimals0, decimals1).await?;
            Ok(PriceAndRaw {
                price,
                reserve0: Some(r0),
                reserve1: Some(r1),
                sqrt_price_x96: None,
            })
        }
        PoolKind::V3 => {
            let (price, sqrt_x96) = fetch_v3_price(provider, pool, decimals0, decimals1).await?;
            Ok(PriceAndRaw {
                price,
                reserve0: None,
                reserve1: None,
                sqrt_price_x96: Some(sqrt_x96),
            })
        }
    }
}

async fn fetch_v2_price(
    provider: &Provider<Ws>,
    pool: &Address,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, f64, f64), MarketScannerError> {
    let res = eth_call(provider, *pool, SELECTOR_GET_RESERVES).await?;
    if res.len() < 64 {
        return Err(MarketScannerError::WsRpcError(
            "getReserves response too short".into(),
        ));
    }
    let r0 = U256::from_big_endian(&res[0..32]).as_u128() as f64 / 10f64.powi(decimals0 as i32);
    let r1 = U256::from_big_endian(&res[32..64]).as_u128() as f64 / 10f64.powi(decimals1 as i32);
    if r0 == 0.0 {
        return Err(MarketScannerError::WsRpcError("zero reserve0".into()));
    }
    Ok((r1 / r0, r0, r1))
}

async fn fetch_v3_price(
    provider: &Provider<Ws>,
    pool: &Address,
    decimals0: u8,
    decimals1: u8,
) -> Result<(f64, u128), MarketScannerError> {
    let res = eth_call(provider, *pool, SELECTOR_SLOT0).await?;
    if res.len() < 32 {
        return Err(MarketScannerError::WsRpcError(
            "slot0 response too short".into(),
        ));
    }
    let sqrt_price_x96 = U256::from_big_endian(&res[0..32]).as_u128();
    let sqrt_f = sqrt_price_x96 as f64;
    let q96 = 2f64.powi(96);
    let price = (sqrt_f / q96).powi(2);
    let decimals_adj = 10f64.powi((decimals1 as i32) - (decimals0 as i32));
    Ok((price * decimals_adj, sqrt_price_x96))
}
