//! DEX pool listener V3 test (swap-event only).
//!
//! Set RPC via env, then run:
//!
//!   POOL_LISTENER_RPC_WS=wss://... cargo test pool_listener_v3 -- --nocapture
//!
//! Pool address and chain are fixed in this file (edit if needed).

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
const POOL_ADDRESS: &str = "0x6fe9E9de56356F7eDBfcBB29FAB7cd69471a4869"; // USDT/BNB Uniswap V3 on BNB chain

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
        "USDT",
        "USDT",
        18,
        ChainId::BSC,
    );
    let token1 = Token::create(
        "0x0000000000000000000000000000000000000000",
        "BNB",
        "BNB",
        18,
        ChainId::BSC,
    );
    let pool = PoolWithTokens {
        pool_address: POOL_ADDRESS.to_string(),
        pool_kind: PoolKind::V3,
        pool_id: None,
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
            if count >= 5000 {
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
async fn pool_listener_v3_on_swap_event() {
    println!("\n=== Pool listener V3 — Swap events only ===\n");
    let Some(count) = run_listener(45).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("No swap in pool during timeout (normal for quiet pools)");
    }
}
