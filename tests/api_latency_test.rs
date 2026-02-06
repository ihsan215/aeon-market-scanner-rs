//! Her bir borsa/API isteğinin süresini ayrı ayrı ölçen test.
//! Çalıştırmak için: cargo test api_latency -- --nocapture --test-threads=1

mod scanner_common;

use aeon_market_scanner_rs::{
    Binance, Bitfinex, Bitget, Btcturk, Bybit, CEXTrait, Coinbase, Cryptocom, DEXTrait, Gateio,
    Htx, Kraken, Kucoin, KyberSwap, Mexc, OKX, Upbit,
};
use scanner_common::{
    create_base_eth, create_base_usdc, create_bsc_bnb, create_bsc_usdt, create_eth_eth,
    create_eth_usdt,
};
use std::time::Instant;

const CEX_SYMBOL: &str = "BTCUSDT";
const DEX_QUOTE_AMOUNT: f64 = 1000.0;

macro_rules! measure_cex {
    ($name:expr, $exchange:expr, $symbol:expr) => {{
        let start = Instant::now();
        let result = $exchange.get_price($symbol).await;
        let elapsed = start.elapsed();
        (stringify!($name), elapsed, result)
    }};
}

#[tokio::test]
async fn test_api_latency_cex() {
    println!("\n=== CEX API İstek Süreleri (get_price) ===\n");

    let mut results: Vec<(&str, std::time::Duration, bool)> = Vec::new();

    // Binance
    let (name, elapsed, result) = measure_cex!(Binance, Binance::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Bybit
    let (name, elapsed, result) = measure_cex!(Bybit, Bybit::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // MEXC
    let (name, elapsed, result) = measure_cex!(Mexc, Mexc::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // OKX
    let (name, elapsed, result) = measure_cex!(OKX, OKX::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Gateio
    let (name, elapsed, result) = measure_cex!(Gateio, Gateio::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Kucoin
    let (name, elapsed, result) = measure_cex!(Kucoin, Kucoin::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Bitget
    let (name, elapsed, result) = measure_cex!(Bitget, Bitget::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Btcturk
    let (name, elapsed, result) = measure_cex!(Btcturk, Btcturk::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Htx
    let (name, elapsed, result) = measure_cex!(Htx, Htx::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Coinbase
    let (name, elapsed, result) = measure_cex!(Coinbase, Coinbase::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Kraken
    let (name, elapsed, result) = measure_cex!(Kraken, Kraken::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Bitfinex
    let (name, elapsed, result) = measure_cex!(Bitfinex, Bitfinex::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Upbit (BTCUSD formatı kullanılıyor testlerde; BTCUSDT da denenebilir)
    let (name, elapsed, result) = measure_cex!(Upbit, Upbit::new(), "BTCUSD");
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    // Cryptocom
    let (name, elapsed, result) = measure_cex!(Cryptocom, Cryptocom::new(), CEX_SYMBOL);
    let ok = result.is_ok();
    results.push((name, elapsed, ok));
    println!(
        "{:12} {:8} ms  {}",
        name,
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );

    println!("\n--- Özet ---");
    let total_ms: u128 = results.iter().map(|(_, d, _)| d.as_millis()).sum();
    let success_count = results.iter().filter(|(_, _, ok)| *ok).count();
    println!(
        "Toplam: {} ms, Başarılı: {}/{}",
        total_ms,
        success_count,
        results.len()
    );
}

#[tokio::test]
async fn test_api_latency_dex() {
    println!("\n=== DEX API İstek Süreleri (get_price - KyberSwap) ===\n");

    let exchange = KyberSwap::new();
    let mut results: Vec<(&str, std::time::Duration, bool)> = Vec::new();

    // KyberSwap Ethereum - ETH/USDT
    let base = create_eth_eth();
    let quote = create_eth_usdt();
    let start = Instant::now();
    let result = exchange
        .get_price(&base, &quote, DEX_QUOTE_AMOUNT)
        .await;
    let elapsed = start.elapsed();
    let ok = result.is_ok();
    results.push(("KyberSwap ETH", elapsed, ok));
    println!(
        "{:20} {:8} ms  {}  (Ethereum ETH/USDT)",
        "KyberSwap ETH",
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );
    if let Err(e) = &result {
        println!("  Hata: {:?}", e);
    }

    // KyberSwap Base - ETH/USDC
    let base = create_base_eth();
    let quote = create_base_usdc();
    let start = Instant::now();
    let result = exchange
        .get_price(&base, &quote, DEX_QUOTE_AMOUNT)
        .await;
    let elapsed = start.elapsed();
    let ok = result.is_ok();
    results.push(("KyberSwap Base", elapsed, ok));
    println!(
        "{:20} {:8} ms  {}  (Base ETH/USDC)",
        "KyberSwap Base",
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );
    if let Err(e) = &result {
        println!("  Hata: {:?}", e);
    }

    // KyberSwap BSC - BNB/USDT
    let base = create_bsc_bnb();
    let quote = create_bsc_usdt();
    let start = Instant::now();
    let result = exchange
        .get_price(&base, &quote, DEX_QUOTE_AMOUNT)
        .await;
    let elapsed = start.elapsed();
    let ok = result.is_ok();
    results.push(("KyberSwap BSC", elapsed, ok));
    println!(
        "{:20} {:8} ms  {}  (BSC BNB/USDT)",
        "KyberSwap BSC",
        elapsed.as_millis(),
        if ok { "OK" } else { "FAIL" }
    );
    if let Err(e) = &result {
        println!("  Hata: {:?}", e);
    }

    println!("\n--- DEX Özet ---");
    let total_ms: u128 = results.iter().map(|(_, d, _)| d.as_millis()).sum();
    let success_count = results.iter().filter(|(_, _, ok)| *ok).count();
    println!(
        "Toplam: {} ms, Başarılı: {}/{}",
        total_ms,
        success_count,
        results.len()
    );
}
