//! DEX pool listener V4 test on BSC (PancakeSwap Infinity / V4-style).
//!
//! PoolManager and pool id are fixed in the test; only RPC requires env.
//!
//!   POOL_LISTENER_RPC_WS=wss://bsc-ws... cargo test pool_listener_v4 -- --nocapture
//!
//! Uses PoolKind::V4Pancake (PancakeSwap Infinity Swap event signature). Pool id = topic1 from BSCScan.

use aeon_market_scanner_rs::{
    ChainId, DexPrice, PoolKind, PoolWithTokens, PriceDirection, Token, stream_pool_prices,
};

fn print_update(n: u32, u: &DexPrice) {
    println!(
        "Update #{}: price={} direction={:?} | sqrt_price_x96={:?} | block={} ts={} | symbol={:?}",
        n, u.price, u.direction, u.sqrt_price_x96, u.block_number, u.timestamp, u.symbol
    );
}

const CHAIN_ID: u64 = 56;
/// PancakeSwap Infinity CLPoolManager on BSC
const POOL_MANAGER_BSC: &str = "0xa0FfB9c1CE1Fe56963B0321B32E7A0302114058b";

// BNB/USDT Pool ID (64 hex chars = 32 bytes)
const POOL_ID_BSC_HEX: &str = "d37aa0f0d66ad670279f6b89325c88bdff17d0265144762fb01f54fca9779944";

fn pool_id_bsc() -> Option<[u8; 32]> {
    let mut arr = [0u8; 32];
    for (i, chunk) in POOL_ID_BSC_HEX.as_bytes().chunks(2).enumerate() {
        arr[i] = u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap();
    }
    Some(arr)
}

fn rpc_ws() -> Option<String> {
    let _ = dotenvy::dotenv();
    let s = std::env::var("POOL_LISTENER_RPC_WS").ok()?;
    if s.is_empty() {
        return None;
    }
    Some(s)
}

async fn run_listener(timeout_secs: u64) -> Option<u32> {
    let rpc_ws = rpc_ws()?;

    let token0 = Token::create(
        "0x0000000000000000000000000000000000000000",
        "BNB",
        "BNB",
        18,
        ChainId::BSC,
    );
    let token1 = Token::create(
        "0x0000000000000000000000000000000000000000",
        "USDT",
        "USDT",
        18,
        ChainId::BSC,
    );
    let pool = PoolWithTokens {
        pool_address: POOL_MANAGER_BSC.to_string(),
        pool_kind: PoolKind::V4Infinity,
        pool_id: pool_id_bsc(),
        token0,
        token1,
        price_direction: PriceDirection::Token1PerToken0,
    };

    let mut rx = stream_pool_prices(rpc_ws, CHAIN_ID, vec![pool], 0, 5000)
        .await
        .expect("stream_pool_prices");

    let timeout = std::time::Duration::from_secs(timeout_secs);
    let mut count = 0u32;

    let result = tokio::time::timeout(timeout, async {
        while let Some(update) = rx.recv().await {
            count += 1;
            print_update(count, &update);
            if count >= 10 {
                break;
            }
        }
    })
    .await;

    match result {
        Ok(()) => {}
        Err(_) => println!("Timeout after {:?} (received {} updates)", timeout, count),
    }
    Some(count)
}

#[tokio::test]
async fn pool_listener_v4_bsc_on_swap_event() {
    println!("\n=== Pool listener V4 BSC (PancakeSwap Infinity) — Swap events only ===\n");
    let Some(count) = run_listener(45).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS (BSC)");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("No swap in pool during timeout (normal for quiet pools or wrong pool id)");
    }
}

// --- Uniswap V4 on BSC (BNB/USDT) ---

/// Uniswap V4 PoolManager on BSC
const UNISWAP_V4_POOL_MANAGER_BSC: &str = "0x28e2Ea090877bF75740558f6BFB36A5ffeE9e9dF";
/// USDC/USDT pool id (Uniswap V4 on BSC)
const POOL_ID_UNISWAP_BSC_HEX: &str =
    "8321c1f53959b14ece4b5400e60aeac59e7b6b8bac446f2f0a89b9e84e68a08a";

fn pool_id_uniswap_bsc() -> [u8; 32] {
    let mut arr = [0u8; 32];
    for i in 0..32 {
        arr[i] = u8::from_str_radix(&POOL_ID_UNISWAP_BSC_HEX[i * 2..i * 2 + 2], 16).unwrap();
    }
    arr
}

async fn run_listener_uniswap_v4_bsc(timeout_secs: u64) -> Option<u32> {
    let rpc_ws = rpc_ws()?;

    let token0 = Token::create(
        "0x0000000000000000000000000000000000000000",
        "USDC",
        "USDC",
        18,
        ChainId::BSC,
    );
    let token1 = Token::create(
        "0x0000000000000000000000000000000000000000",
        "USDT",
        "USDT",
        18,
        ChainId::BSC,
    );
    let pool = PoolWithTokens {
        pool_address: UNISWAP_V4_POOL_MANAGER_BSC.to_string(),
        pool_kind: PoolKind::V4,
        pool_id: Some(pool_id_uniswap_bsc()),
        token0,
        token1,
        price_direction: PriceDirection::Token0PerToken1,
    };

    let mut rx = stream_pool_prices(rpc_ws, CHAIN_ID, vec![pool], 0, 5000)
        .await
        .expect("stream_pool_prices");

    let timeout = std::time::Duration::from_secs(timeout_secs);
    let mut count = 0u32;

    let result = tokio::time::timeout(timeout, async {
        while let Some(update) = rx.recv().await {
            count += 1;
            print_update(count, &update);
            if count >= 10 {
                break;
            }
        }
    })
    .await;

    match result {
        Ok(()) => {}
        Err(_) => println!("Timeout after {:?} (received {} updates)", timeout, count),
    }
    Some(count)
}

#[tokio::test]
async fn pool_listener_v4_bsc_uniswap_on_swap_event() {
    println!("\n=== Pool listener V4 BSC (Uniswap V4) BNB/USDT — Swap events only ===\n");
    let Some(count) = run_listener_uniswap_v4_bsc(45).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS (BSC)");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("No swap in pool during timeout");
    }
}
