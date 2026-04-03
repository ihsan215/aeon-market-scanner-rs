//! MEXC private account integration test.
//! Run: cargo test mexc_account -- --nocapture

use aeon_market_scanner_rs::Mexc;

fn has_mexc_credentials() -> bool {
    let _ = dotenvy::dotenv();

    matches!(std::env::var("MEXC_API_KEY"), Ok(value) if !value.is_empty())
        && matches!(std::env::var("MEXC_API_SECRET"), Ok(value) if !value.is_empty())
}

#[tokio::test]
async fn mexc_account_balance_then_listen_orders_for_30_seconds() {
    if !has_mexc_credentials() {
        println!("Skipping: set MEXC_API_KEY and MEXC_API_SECRET");
        return;
    }

    println!("\n=== MEXC private account test: balance + order stream ===\n");

    let mexc = Mexc::with_credentials(
        std::env::var("MEXC_API_KEY").unwrap(),
        std::env::var("MEXC_API_SECRET").unwrap(),
    );

    let balances = mexc
        .get_account_balances()
        .await
        .expect("MEXC account balances");

    println!(
        "Account type: {:?} | permissions: {:?} | balances: {}",
        balances.account_type,
        balances.permissions,
        balances.balances.len()
    );

    for balance in balances.balances.iter().take(20) {
        println!(
            "{} free={} locked={} available={:?}",
            balance.asset, balance.free, balance.locked, balance.available
        );
    }

    let mut rx = mexc
        .stream_spot_order_updates(5, 5_000)
        .await
        .expect("MEXC private order stream");

    let timeout = std::time::Duration::from_secs(10);
    let mut count = 0u32;

    let result = tokio::time::timeout(timeout, async {
        while let Some(order) = rx.recv().await {
            count += 1;
            let display = order.to_display();
            println!(
                "#{} pair={} time={} type={} side={} action={} avg_filled_price={} price={} executed={} quantity={:?} order_amount={:?} total={} status={} order_id={} client_id={}",
                count,
                display.trading_pair,
                display.time,
                display.order_type,
                display.side,
                display.event_action,
                display.average_filled_price,
                display.price,
                display.executed,
                display.quantity,
                display.order_amount,
                display.total,
                display.status,
                order.order_id,
                order.client_id
            );
        }
    })
    .await;

    match result {
        Ok(()) => println!("Order stream closed after {} updates", count),
        Err(_) => println!(
            "Order stream timed out after {:?} with {} updates",
            timeout, count
        ),
    }
}
