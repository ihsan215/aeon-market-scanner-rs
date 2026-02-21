//! DEX pool listener V2 test.
//!
//! Set only RPC via env, then run:
//!
//!   POOL_LISTENER_RPC_WS=wss://... cargo test pool_listener_v2 -- --nocapture
//!
//! Pool address and chain are fixed in this file (edit if needed).

use aeon_market_scanner_rs::{
    ListenMode, PoolKind, PoolListenerConfig, PoolPriceUpdate, PriceDirection, load_dotenv,
    stream_pool_prices,
};

fn print_update(n: u32, u: &PoolPriceUpdate) {
    println!(
        "Update #{}: price={} direction={:?} | reserve0={:?} reserve1={:?} | block={} ts={} | symbol={:?}",
        n, u.price, u.direction, u.reserve0, u.reserve1, u.block_number, u.timestamp, u.symbol
    );
}

const CHAIN_ID: u64 = 56;
const POOL_ADDRESS: &str = "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE"; // PancakeSwap V2 BNB/USDT on BNB chain

fn rpc_ws() -> Option<String> {
    load_dotenv();
    let s = std::env::var("POOL_LISTENER_RPC_WS").ok()?;
    if s.is_empty() {
        return None;
    }
    Some(s)
}

async fn run_listener(listen_mode: ListenMode, timeout_secs: u64) -> Option<u32> {
    let rpc_ws = rpc_ws()?;

    let config = PoolListenerConfig {
        rpc_ws_url: rpc_ws.clone(),
        chain_id: CHAIN_ID,
        pool_address: POOL_ADDRESS.to_string(),
        pool_kind: PoolKind::V2,
        listen_mode,
        price_direction: PriceDirection::Token0PerToken1,
        symbol: Some("BNBUSDT".to_string()),
        reconnect_attempts: 0,
        reconnect_delay_ms: 5000,
    };

    let mut rx = stream_pool_prices(config)
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
    println!("\n=== Pool listener V2 — OnSwapEvent ===\n");
    let Some(count) = run_listener(ListenMode::OnSwapEvent, 45).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };
    println!("\nTotal updates: {}", count);
    if count == 0 {
        println!("OnSwapEvent: no swap in pool during timeout (normal for quiet pools)");
    }
}

#[tokio::test]
async fn pool_listener_v2_every_block() {
    println!("\n=== Pool listener V2 — EveryBlock ===\n");
    let Some(count) = run_listener(ListenMode::EveryBlock, 30).await else {
        println!("Skipping: set POOL_LISTENER_RPC_WS");
        return;
    };
    println!("\nTotal updates: {}", count);
    assert!(
        count >= 1,
        "EveryBlock should yield at least one update; got {}",
        count
    );
}
