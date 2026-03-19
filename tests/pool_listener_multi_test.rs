//! DEX pool listener multi-pool test (swap-event only).
//!
//! Set RPC via env, then run:
//!
//!   POOL_LISTENER_RPC_WS=wss://... cargo test pool_listener_multi -- --nocapture
//!
//! Pools and chain are fixed in this file (edit if needed).

use aeon_market_scanner_rs::{
    ChainId, DexPrice, PoolKind, PoolWithTokens, PriceDirection, Token, stream_pool_prices,
};
use std::collections::HashSet;

fn print_update(n: u32, u: &DexPrice) {
    println!(
        "Multi update #{}: pool={} kind={:?} bid={} ask={} mid={} direction={:?} | block={} ts={} | symbol={:?}",
        n,
        u.pool_address,
        u.pool_kind,
        u.bid,
        u.ask,
        u.mid,
        u.direction,
        u.block_number,
        u.timestamp,
        u.symbol
    );
}

const CHAIN_ID: ChainId = ChainId::BSC;
const POOL_V2_ADDRESS: &str = "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE"; // PancakeSwap V2 BNB/USDT on BNB chain
const POOL_V3_ADDRESS: &str = "0x6fe9E9de56356F7eDBfcBB29FAB7cd69471a4869"; // USDT/BNB Uniswap V3 on BNB chain
const POOL_MANAGER_V4_BSC: &str = "0x28e2Ea090877bF75740558f6BFB36A5ffeE9e9dF"; // Uniswap V4 PoolManager on BSC
const POOL_ID_UNISWAP_BSC_HEX: &str =
    "8321c1f53959b14ece4b5400e60aeac59e7b6b8bac446f2f0a89b9e84e68a08a"; // USDC/USDT pool id on BSC

fn rpc_ws() -> Option<String> {
    let _ = dotenvy::dotenv();
    let s = std::env::var("POOL_LISTENER_RPC_WS").ok()?;
    if s.is_empty() {
        return None;
    }
    Some(s)
}

fn pool_id_uniswap_bsc() -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = u8::from_str_radix(&POOL_ID_UNISWAP_BSC_HEX[i * 2..i * 2 + 2], 16).unwrap();
    }
    arr
}

#[tokio::test]
async fn pool_listener_multi_on_swap_event() {
    println!("\n=== Pool listener multi-pool — Swap events only ===\n");
    let Some(rpc_ws) = rpc_ws() else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };

    // Reuse known busy pools on BSC: one V2, one V3, one V4.
    let token_bnb = Token::create(
        "0x0000000000000000000000000000000000000000",
        "BNB",
        "BNB",
        18,
        ChainId::BSC,
    );
    let token_usdt = Token::create(
        "0x0000000000000000000000000000000000000000",
        "USDT",
        "USDT",
        18,
        ChainId::BSC,
    );
    let token_usdc = Token::create(
        "0x0000000000000000000000000000000000000000",
        "USDC",
        "USDC",
        18,
        ChainId::BSC,
    );

    // PancakeSwap V2 BNB/USDT on BSC
    let pool_v2 = PoolWithTokens {
        pool_address: POOL_V2_ADDRESS.to_string(),
        pool_kind: PoolKind::V2Pancake,
        pool_id: None,
        token0: token_bnb.clone(),
        token1: token_usdt.clone(),
        price_direction: PriceDirection::Token0PerToken1,
        fee_bps: 250,
    };

    // Uniswap V3 USDT/BNB on BSC
    let pool_v3 = PoolWithTokens {
        pool_address: POOL_V3_ADDRESS.to_string(),
        pool_kind: PoolKind::V3Uniswap,
        pool_id: None,
        token0: token_usdt.clone(),
        token1: token_bnb.clone(),
        price_direction: PriceDirection::Token0PerToken1,
        fee_bps: 500,
    };

    // Uniswap V4 USDC/USDT on BSC
    let pool_v4 = PoolWithTokens {
        pool_address: POOL_MANAGER_V4_BSC.to_string(),
        pool_kind: PoolKind::V4Uniswap,
        pool_id: Some(pool_id_uniswap_bsc()),
        token0: token_usdc,
        token1: token_usdt,
        price_direction: PriceDirection::Token0PerToken1,
        fee_bps: 67,
    };

    let pools = vec![pool_v2, pool_v3, pool_v4];

    let mut rx = stream_pool_prices(rpc_ws, CHAIN_ID, pools, 0, 5000, None)
        .await
        .expect("stream_pool_prices");

    let timeout = std::time::Duration::from_secs(60);
    let mut count = 0u32;
    let mut seen_pools: HashSet<String> = HashSet::new();

    let result = tokio::time::timeout(timeout, async {
        while let Some(update) = rx.recv().await {
            count += 1;
            print_update(count, &update);
            seen_pools.insert(update.pool_address.clone());
            if count >= 5 {
                break;
            }
        }
    })
    .await;

    match result {
        Ok(()) => {}
        Err(_) => println!(
            "Timeout after {:?} (received {} updates, pools={:?})",
            timeout, count, seen_pools
        ),
    }

    if count == 0 {
        println!("No swaps observed in multi-pool test window (normal for very quiet pools)");
    }
}
