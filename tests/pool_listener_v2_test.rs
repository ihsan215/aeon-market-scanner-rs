//! DEX pool listener V2 test (Sync-event only).
//!
//! Set RPC via env, then run:
//!
//!   POOL_LISTENER_RPC_WS=wss://... cargo test pool_listener_v2 -- --nocapture
//!
//! Pool address and chain are fixed in this file (edit if needed).

use aeon_market_scanner_rs::{
    ChainId, DexPrice, PoolKind, PoolWithTokens, PriceDirection, Token, stream_pool_prices,
};

fn print_update(n: u32, u: &DexPrice) {
    println!(
        "Update #{}: bid={} ask={} mid={} direction={:?} | block={} ts={} | symbol={:?}",
        n, u.bid, u.ask, u.mid, u.direction, u.block_number, u.timestamp, u.symbol
    );
}

const CHAIN_ID: ChainId = ChainId::BSC;
const POOL_ADDRESS: &str = "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE"; // PancakeSwap V2 BNB/USDT on BNB chain
const POOL_ADDRESS_UNISWAP: &str = "0x8a1Ed8e124fdFBD534bF48baF732E26db9Cc0Cf4"; // Replace with a Uniswap V2 pair address if needed

fn rpc_ws() -> Option<String> {
    let _ = dotenvy::dotenv();
    let s = std::env::var("POOL_LISTENER_RPC_WS").ok()?;
    if s.is_empty() {
        return None;
    }
    Some(s)
}

async fn run_listener(
    timeout_secs: u64,
    pool_address: &str,
    pool_kind: PoolKind,
    fee_bps: u32,
) -> Option<u32> {
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
        pool_address: pool_address.to_string(),
        pool_kind,
        pool_id: None,
        token0,
        token1,
        price_direction: PriceDirection::Token0PerToken1,
        fee_bps,
    };

    let mut rx = stream_pool_prices(rpc_ws, CHAIN_ID, vec![pool], 0, 5000, None)
        .await
        .expect("stream_pool_prices");

    let timeout = std::time::Duration::from_secs(timeout_secs);
    let mut count = 0u32;

    let result = tokio::time::timeout(timeout, async {
        while let Some(update) = rx.recv().await {
            count += 1;
            print_update(count, &update);
            if count >= 5 {
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
async fn pool_listener_v2_on_swap_event() {
    println!("\n=== Pool listener V2 Pancake — Sync events only ===\n");
    let Some(count) = run_listener(45, POOL_ADDRESS, PoolKind::V2Pancake, 25).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("No sync in pool during timeout (normal for quiet pools)");
    }
}

#[tokio::test]
async fn pool_listener_v2_uniswap_on_sync_event() {
    println!("\n=== Pool listener V2 Uniswap — Sync events only ===\n");
    let Some(count) = run_listener(45, POOL_ADDRESS_UNISWAP, PoolKind::V2Uniswap, 30).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("No sync in pool during timeout (normal for quiet pools)");
    }
}
