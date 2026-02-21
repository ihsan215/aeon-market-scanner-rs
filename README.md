# aeon-market-scanner-rs

A Rust crate for fetching **CEX** and **DEX** prices and finding **arbitrage opportunities**.

- REST price fetching (`get_price`)
- CEX WebSocket streams (`stream_price_websocket`) with configurable reconnect (attempts + delay in ms)
- **DEX pool price listener**: Uniswap V2/V3 pool prices over WebSocket RPC (`stream_pool_prices`)
- Arbitrage scanning: one-shot REST (`scan_arbitrage_opportunities`) or live WebSocket (`scan_arbitrage_from_websockets`)
- Fee overrides (VIP/custom tiers) and optional DEX legs (KyberSwap)

> **Crate:** [crates.io/crates/aeon-market-scanner-rs](https://crates.io/crates/aeon-market-scanner-rs) · **Docs:** [docs.rs/aeon-market-scanner-rs](https://docs.rs/aeon-market-scanner-rs)  
> Import: `aeon_market_scanner_rs`

## Supported exchanges

### CEX (centralized exchanges)

| Exchange   | REST (`get_price`) | WebSocket (`supports_websocket()`) |
| ---------- | -----------------: | ---------------------------------: |
| Binance    |          supported |                          supported |
| Bybit      |          supported |                          supported |
| MEXC       |          supported |                          supported |
| OKX        |          supported |                          supported |
| Gateio     |          supported |                          supported |
| KuCoin     |          supported |                          supported |
| Bitget     |          supported |                          supported |
| Coinbase   |          supported |                          supported |
| Kraken     |          supported |                          supported |
| Bitfinex   |          supported |                          supported |
| Upbit      |          supported |                          supported |
| Crypto.com |          supported |                          supported |
| BtcTurk    |          supported |                      not supported |
| HTX        |          supported |                      not supported |

### DEX

| Component             |      REST |     WebSocket | Notes                                                                                                                                                               |
| --------------------- | --------: | ------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| KyberSwap             | supported | not supported | Aggregator; chains: `ethereum`, `bsc`, `polygon`, `avalanche`, `arbitrum`, `optimism`, `base`, `linea`, `mantle`, `plasma`, `unichain`, `sonic`, `ronin`, `hyprevm` |
| Pool listener (V2/V3) |       n/a |     supported | Single-pool price stream over your WebSocket RPC; any EVM chain                                                                                                     |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aeon-market-scanner-rs = "0.4"
tokio = { version = "1", features = ["full"] }
```

Or pin the exact version:

```toml
aeon-market-scanner-rs = "0.4.0"
```

Then run `cargo build`.

## Quickstart: fetch a CEX price (REST)

```rust
use aeon_market_scanner_rs::{Binance, CEXTrait};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let price = Binance::new().get_price("BTCUSDT").await?;

    println!(
        "{} bid={} ask={} mid={}",
        price.symbol, price.bid_price, price.ask_price, price.mid_price
    );

    Ok(())
}
```

## Stream CEX prices via WebSocket (with reconnect)

All WebSocket-enabled CEX implementations expose:

```text
stream_price_websocket(symbols, reconnect_attempts, reconnect_delay_ms)
```

- `reconnect_attempts`: `0` = no reconnect; `n` = up to n reconnects (1 initial run + n retries)
- `reconnect_delay_ms`: milliseconds to wait before each reconnect (0 is treated as 1000)

Example:

```rust,no_run
use aeon_market_scanner_rs::{Binance, CEXTrait};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let exchange = Binance::new();

    if !exchange.supports_websocket() {
        eprintln!("This exchange does not support WebSocket streaming");
        return Ok(());
    }

    let mut rx = exchange
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], 10, 5000)
        .await?;

    while let Some(update) = rx.recv().await {
        println!(
            "[{:?}] {} bid={} ask={}",
            update.exchange, update.symbol, update.bid_price, update.ask_price
        );
    }

    Ok(())
}
```

## DEX pool price listener (Uniswap V2 / V3)

Stream live prices from a single Uniswap V2 or V3 style pool over WebSocket RPC. Useful for on-chain price feeds without polling.

```rust,no_run
use aeon_market_scanner_rs::{
    stream_pool_prices, load_dotenv,
    ListenMode, PoolKind, PoolListenerConfig, PriceDirection,
};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    load_dotenv();
    let rpc_ws = std::env::var("POOL_LISTENER_RPC_WS").expect("POOL_LISTENER_RPC_WS");

    let config = PoolListenerConfig {
        rpc_ws_url: rpc_ws,
        chain_id: 56,
        pool_address: "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE".to_string(),
        pool_kind: PoolKind::V2,
        listen_mode: ListenMode::EveryBlock,
        price_direction: PriceDirection::Token1PerToken0,
        symbol: Some("BNBUSDT".to_string()),
        reconnect_attempts: 3,
        reconnect_delay_ms: 5000,
    };

    let mut rx = stream_pool_prices(config).await?;
    while let Some(update) = rx.recv().await {
        println!("price={} block={} reserve0={:?} reserve1={:?}",
            update.price, update.block_number, update.reserve0, update.reserve1);
    }
    Ok(())
}
```

- **ListenMode**: `EveryBlock` (emit on each new block from RPC) or `OnSwapEvent` (only when the pool emits a Swap).
- **PriceDirection**: `Token1PerToken0` (e.g. USDT per BNB) or `Token0PerToken1`.
- **Reconnect**: `reconnect_attempts` = 0 to disable; n = up to n reconnects. `reconnect_delay_ms` = delay between attempts (0 → 1000 ms).
- V2 pools expose `reserve0` / `reserve1`; V3 pools expose `sqrt_price_x96`.

## Scan arbitrage opportunities (CEX-only)

```rust,no_run
use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let symbol = "BTCUSDT";

    let opportunities = ArbitrageScanner::scan_arbitrage_opportunities(
        symbol,
        &[
            CexExchange::Binance,
            CexExchange::OKX,
            CexExchange::Bybit,
            CexExchange::Kucoin,
        ],
        None,
        None,
        None,
        None,
        None,
    )
    .await?;

    for opp in opportunities.iter().take(5) {
        println!(
            "{} -> {} {} spread={:.4} ({:.3}%) qty={:.6}",
            opp.source_exchange,
            opp.destination_exchange,
            opp.symbol,
            opp.spread,
            opp.spread_percentage,
            opp.executable_quantity
        );
    }

    Ok(())
}
```

## Scan arbitrage opportunities (CEX + DEX)

If you want to include KyberSwap routes, pass the DEX list + tokens. Example below uses **Ethereum mainnet** WETH/USDT addresses.

```rust,no_run
use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange, DexAggregator};
use aeon_market_scanner_rs::dex::chains::{ChainId, Token};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let symbol = "ETHUSDT";

    let weth = Token::create(
        "0xC02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2",
        "Wrapped Ether",
        "WETH",
        18,
        ChainId::ETHEREUM,
    );
    let usdt = Token::create(
        "0xdAC17F958D2ee523a2206206994597C13D831ec7",
        "Tether USD",
        "USDT",
        6,
        ChainId::ETHEREUM,
    );

    let quote_amount = 1_000.0; // in quote token units (e.g., 1000 USDT)

    let opportunities = ArbitrageScanner::scan_arbitrage_opportunities(
        symbol,
        &[CexExchange::Binance, CexExchange::OKX],
        Some(&[DexAggregator::KyberSwap]),
        Some(&weth),
        Some(&usdt),
        Some(quote_amount),
        None,
    )
    .await?;

    println!("Found {} opportunities", opportunities.len());
    Ok(())
}
```

## Scan arbitrage opportunities from WebSocket streams

Connect to CEX WebSocket feeds and continuously receive arbitrage opportunity snapshots:

```rust,no_run
use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let mut rx = ArbitrageScanner::scan_arbitrage_from_websockets(
        &["BTCUSDT", "ETHUSDT"],
        &[CexExchange::Binance, CexExchange::OKX, CexExchange::Bybit],
        None,
        10,   // reconnect_attempts
        5000, // reconnect_delay_ms
    )
    .await?;

    while let Some(opps) = rx.recv().await {
        for o in opps.iter().take(5) {
            println!(
                "{} -> {} {} spread={:.4} ({:.3}%)",
                o.source_exchange, o.destination_exchange, o.symbol,
                o.spread, o.spread_percentage
            );
        }
    }

    Ok(())
}
```

Exchanges that do not support WebSocket are skipped. The receiver emits opportunity snapshots (sorted by profitability) whenever new prices arrive.

## Fees / commissions

Arbitrage opportunities are evaluated using **effective prices** that account for taker fees:

- **Buy side**: effective ask = \(ask \times (1 + fee)\)
- **Sell side**: effective bid = \(bid \times (1 - fee)\)

This means the reported spread/profitability is **fee-aware** by default. Fee rates are defined as default-tier spot **taker** fees in `src/common/commission.rs` (VIP/volume discounts are not applied).

### Override fee rates (VIP / custom tiers)

If you want to use your own fee rates (e.g. VIP tier), create `FeeOverrides` and pass it into `scan_arbitrage_opportunities(...)`.

```rust,no_run
use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange, FeeOverrides};

let overrides = FeeOverrides::default()
    .with_cex_taker_fee(CexExchange::Binance, 0.00075) // 0.075%
    .with_cex_taker_fee(CexExchange::OKX, 0.0008);     // 0.08%

let opportunities = ArbitrageScanner::scan_arbitrage_opportunities(
    "BTCUSDT",
    &[CexExchange::Binance, CexExchange::OKX],
    None,
    None,
    None,
    None,
    Some(&overrides),
)
.await?;
# let _ = opportunities;
```

### Read fee rates programmatically

Fee rates are exposed as `f64` decimals (e.g. `0.001` = `0.1%`):

```rust
use aeon_market_scanner_rs::{CexExchange, Exchange, fee_rate, taker_fee_rate};

let binance_taker = taker_fee_rate(&CexExchange::Binance);
println!("Binance taker fee = {} ({}%)", binance_taker, binance_taker * 100.0);

let okx_fee = fee_rate(&Exchange::Cex(CexExchange::OKX));
println!("OKX fee (generic) = {} ({}%)", okx_fee, okx_fee * 100.0);
```

## Notes / caveats

- **Public APIs**: this crate uses exchanges' **public REST and (where available) public WebSocket** market data endpoints. No API keys are required for the features in this crate. Usage is still subject to each provider’s rate limits and terms.
- **Network + rate limits**: exchange APIs can rate-limit or temporarily fail; callers should expect errors.
- **Symbols**: most examples use common `BASEQUOTE` format like `BTCUSDT`. Some exchanges may require different formatting internally; the crate normalizes per-exchange.
- **WebSocket streams**: intended for continuous feeds. When the receiver ends (`None`), the underlying connection has closed (and may reconnect if `reconnect_attempts` > 0).
- **Pool listener**: requires a WebSocket RPC URL (e.g. from Alchemy, Infura, or a chain node). Block delivery depends on the RPC; block numbers may not be consecutive.

## License

Licensed under the **Apache License, Version 2.0**. See `LICENSE`.
