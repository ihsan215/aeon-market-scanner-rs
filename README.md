# aeon-market-scanner-rs

A Rust crate for fetching **CEX** and **DEX** prices and finding **arbitrage opportunities**.

- REST price fetching (`get_price`)
- Streaming WebSocket price feeds (`stream_price_websocket`) with reconnect + exponential backoff
- Arbitrage scanning: one-shot REST (`scan_arbitrage_opportunities`) or live WebSocket (`scan_arbitrage_from_websockets`)
- Fee overrides (VIP/custom tiers) and optional DEX legs (KyberSwap)

> **Crate:** [crates.io/crates/aeon-market-scanner-rs](https://crates.io/crates/aeon-market-scanner-rs) · **Docs:** [docs.rs/aeon-market-scanner-rs](https://docs.rs/aeon-market-scanner-rs)  
> Import: `aeon_market_scanner_rs`

## Supported exchanges

### CEX (centralized exchanges)

| Exchange | REST (`get_price`) | WebSocket (`supports_websocket()`) |
|---|---:|---:|
| Binance | supported | supported |
| Bybit | supported | supported |
| MEXC | supported | supported |
| OKX | supported | supported |
| Gateio | supported | supported |
| KuCoin | supported | supported |
| Bitget | supported | supported |
| Coinbase | supported | supported |
| Kraken | supported | supported |
| Bitfinex | supported | supported |
| Upbit | supported | supported |
| Crypto.com | supported | supported |
| BtcTurk | supported | not supported |
| HTX | supported | not supported |

### DEX (decentralized aggregators)

| Aggregator | REST | WebSocket | Supported chains |
|---|---:|---:|---|
| KyberSwap | supported | not supported | `ethereum`, `bsc`, `polygon`, `avalanche`, `arbitrum`, `optimism`, `base`, `linea`, `mantle`, `plasma`, `unichain`, `sonic`, `ronin`, `hyprevm` |

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aeon-market-scanner-rs = "0.3"
tokio = { version = "1", features = ["full"] }
```

Or pin the latest patch:

```toml
aeon-market-scanner-rs = "0.3.0"
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

## Stream prices via WebSocket (with reconnect + max attempts)

All WebSocket-enabled CEX implementations expose:

```text
stream_price_websocket(symbols, reconnect, max_attempts)
```

- `reconnect`: if `true`, automatically reconnect on disconnect/failure with exponential backoff
- `max_attempts`: if `Some(n)`, stop after **n consecutive failed connection attempts**

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
        .stream_price_websocket(&["BTCUSDT", "ETHUSDT"], true, Some(10))
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
        true,
        Some(10),
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
- **WebSocket streams**: intended for continuous feeds. When the receiver ends (`None`), the underlying connection has closed (and may reconnect if enabled).

## License

Licensed under the **Apache License, Version 2.0**. See `LICENSE`.

