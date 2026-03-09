//! DEX pool price listener over WebSocket RPC (ethers-rs).
//!
//! Connects to an Ethereum node via WebSocket, subscribes to Swap events (eth_subscribe "logs"),
//! and emits price updates computed from swap event parameters. Uniswap V2, V3 or V4 style pools.

mod types;
mod utils;
pub use types::{DexPrice, PoolKind, PoolListenerConfig, PoolWithTokens, PriceDirection};

use crate::common::{MarketScannerError, get_timestamp_millis};
use ethers::core::types::{Address, Filter};
use ethers::providers::{Middleware, Provider, Ws};
use futures::stream::{self, StreamExt};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::time::Duration;

/// Subscribe to pool swap events over WebSocket (eth_subscribe "logs" only).
/// One or more pools; all share a single WS connection. Price is computed from swap event parameters.
pub async fn stream_pool_prices(
    rpc_ws_url: String,
    chain_id: u64,
    pools: Vec<PoolWithTokens>,
    reconnect_attempts: u32,
    reconnect_delay_ms: u64,
) -> Result<mpsc::Receiver<DexPrice>, MarketScannerError> {
    let (tx, rx) = mpsc::channel(64);

    tokio::spawn(async move {
        let mut attempt = 0u32;
        loop {
            attempt += 1;
            let _ = run_listener(rpc_ws_url.clone(), chain_id, pools.clone(), tx.clone()).await;
            if reconnect_attempts == 0 || attempt > reconnect_attempts {
                break;
            }
            let delay = Duration::from_millis(reconnect_delay_ms);
            tokio::time::sleep(delay).await;
        }
    });

    Ok(rx)
}

async fn run_listener(
    rpc_ws_url: String,
    chain_id: u64,
    pools: Vec<PoolWithTokens>,
    tx: mpsc::Sender<DexPrice>,
) -> Result<(), MarketScannerError> {
    let ws_provider = Provider::<Ws>::connect(&rpc_ws_url)
        .await
        .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

    let mut streams = Vec::new();

    for pool in pools.into_iter() {
        let pool_addr = Address::from_str(pool.pool_address.trim_start_matches("0x"))
            .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

        let pool_kind = pool.pool_kind;
        let (topic0, topic1) = utils::swap_topic_h256(pool_kind, pool.pool_id.as_ref())?;
        let filter = match topic1 {
            None => Filter::new().address(pool_addr).topic0(topic0),
            Some(t1) => Filter::new().address(pool_addr).topic0(topic0).topic1(t1),
        };

        let log_stream = ws_provider
            .subscribe_logs(&filter)
            .await
            .map_err(|e| MarketScannerError::WsRpcError(e.to_string()))?;

        let pool = Arc::new(pool);
        let symbol = format!("{}{}", pool.token0.symbol, pool.token1.symbol);
        let tagged = log_stream
            .map(move |log| (Arc::clone(&pool), symbol.clone(), log))
            .boxed();
        streams.push(tagged);
    }

    let mut merged = stream::select_all(streams);

    while let Some((pool, symbol, log)) = merged.next().await {
        let pool_address = pool.pool_address.clone();
        let decimals0 = pool.token0.decimal;
        let decimals1 = pool.token1.decimal;
        let price_direction = pool.price_direction;
        let pool_kind = pool.pool_kind;
        let block_number = log.block_number.unwrap_or_default().as_u64();

        let parsed = match pool_kind {
            PoolKind::V2 => {
                if let Ok((p, ..)) = utils::parse_v2_swap_and_price(&log.data, decimals0, decimals1)
                {
                    Some((utils::apply_direction(p, price_direction), None))
                } else {
                    None
                }
            }
            PoolKind::V3 => utils::parse_v3_swap_and_price(&log.data, decimals0, decimals1)
                .ok()
                .map(|(p, sq, ..)| (utils::apply_direction(p, price_direction), Some(sq))),
            PoolKind::V4 | PoolKind::V4Infinity => {
                utils::parse_v4_swap_and_price(&log.data, decimals0, decimals1)
                    .ok()
                    .map(|(p, sq, ..)| (utils::apply_direction(p, price_direction), Some(sq)))
            }
        };

        if let Some((price, sqrt_price_x96)) = parsed {
            let update = DexPrice {
                chain_id,
                pool_address,
                pool_kind,
                price,
                direction: price_direction,
                sqrt_price_x96,
                block_number,
                timestamp: get_timestamp_millis(),
                symbol: Some(symbol),
                pool_id: pool.pool_id,
            };
            if tx.send(update).await.is_err() {
                break;
            }
        }
    }

    Ok(())
}
