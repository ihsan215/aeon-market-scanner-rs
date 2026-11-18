# aeon-market-scanner-rs

A Rust crate for fetching **CEX** and **DEX** prices and finding **arbitrage opportunities**.

- REST price fetching (`get_price`)
- CEX WebSocket streams (`stream_price_websocket`) with configurable reconnect (attempts + delay in ms)
- **MEXC signed API** (optional): account balances, spot orders (market/limit/cancel, **batch** market/limit), and a **private** spot order WebSocket stream — use `Mexc::with_credentials(...)` (keys are **not** read from env inside the crate)
- **DEX pool price listener**: Uniswap V2, V3, V4 and PancakeSwap Infinity (V4-style) pool prices over WebSocket RPC (`stream_pool_prices`); swap-event only, price from event params
- Arbitrage scanning: one-shot REST (`scan_arbitrage_opportunities`) or live WebSocket (`scan_arbitrage_from_websockets` with `ScannerEvent` stream)
- Fee overrides (VIP/custom tiers) and optional DEX legs (KyberSwap)

> **Crate:** [crates.io/crates/aeon-market-scanner-rs](https://crates.io/crates/aeon-market-scanner-rs) · **Docs:** [docs.rs/aeon-market-scanner-rs](https://docs.rs/aeon-market-scanner-rs) · **Live demo:** [aeonfeed.com](https://aeonfeed.com)  
> Import: `aeon_market_scanner_rs`

## Live demo: Aeon Feed (free real-time crypto price API)

[**Aeon Feed**](https://aeonfeed.com) is built on this crate. It streams live top-of-book prices for the top 100 USDT pairs from the 12 WebSocket exchanges above, normalised into one format. It is free and public, with no API key or signup.

- **Live markets:** [aeonfeed.com/markets](https://aeonfeed.com/markets). Best bid/ask, order sizes, cross-exchange spread and per-exchange lag, e.g. [BTC/USDT across 12 exchanges](https://aeonfeed.com/markets/btc-usdt).
- **WebSocket:** `wss://aeonfeed.com/ws`. Per-pair tickers are real-time; cross-exchange aggregates arrive every second. [Docs](https://aeonfeed.com/docs).
- **Exchange formats:** each exchange's native message next to the normalised result, e.g. [Binance](https://aeonfeed.com/exchanges/binance) or [OKX](https://aeonfeed.com/exchanges/okx).

```js
// No API key needed
const ws = new WebSocket("wss://aeonfeed.com/ws?subscribe=ticker:BTCUSDT");
ws.onmessage = (e) => console.log(JSON.parse(e.data)); // { type: "ticker", data: [{ ex, b, a, bq, aq, t, et }] }
```

If you want these streams inside your own Rust service, without a third-party server, use this crate directly (see below).

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

| Component                |      REST |     WebSocket | Notes                                                                                                                                                               |
| ------------------------ | --------: | ------------: | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| KyberSwap                | supported | not supported | Aggregator; chains: `ethereum`, `bsc`, `polygon`, `avalanche`, `arbitrum`, `optimism`, `base`, `linea`, `mantle`, `plasma`, `unichain`, `sonic`, `ronin`, `hyprevm` |
| Pool listener (V2/V3/V4) |       n/a |     supported | Single-pool swap-event stream over WebSocket RPC; V2/V3 (pool address), V4/V4Pancake (PoolManager + pool id); any EVM chain                                         |

### Stocks and fiat currencies (separate modules)

These live outside `cex` / `dex`, have their own price types, and are not part of the arbitrage scanner. Each provider has its own folder (`src/stocks/<provider>/`, `src/fiat/<provider>/`) and implements `StockTrait` / `FiatTrait`, so more providers can be added later.

| Module   | Provider                   | API key            | Data                                                                                |
| -------- | -------------------------- | ------------------ | ----------------------------------------------------------------------------------- |
| `stocks` | Alpaca                     | yes (free account) | Snapshots: bid/ask quote (IEX), last trade, daily close; 200 tickers per request    |
| `stocks` | Tiingo                     | yes (free token)   | Last price (bid/ask only with an IEX data agreement); 100 tickers per request       |
| `stocks` | Finnhub                    | yes (free key)     | Last price; one request per ticker, ~60/min on the free plan                        |
| `stocks` | Yahoo (opt-in, unofficial) | no                 | Last price; feature `unofficial-yahoo`                                              |
| `fiat`   | Frankfurter                | no                 | ECB reference rates (~30 currencies, e.g. EUR/USD, USD/TRY), updated daily           |
| `fiat`   | ExchangeRate-API           | no                 | 160+ currencies, updated daily (attribution required, no redistribution)             |
| `fiat`   | TCMB                       | no                 | ~20 currencies vs TRY with buying/selling (bid/ask), crosses via TRY, working days   |
| `fiat`   | Tiingo FX                  | yes (free token)   | **Real-time** bid/ask for ~140 pairs, many pairs per request (shares the Tiingo quota) |
| `fiat`   | Yahoo (opt-in, unofficial) | no                 | Near-live rates for practically all pairs; feature `unofficial-yahoo`                |

`SP500_SYMBOLS` lists the S&P 500 tickers. `StockPrice.price` is always set (bid/ask midpoint when the provider has a quote, otherwise the last trade); `bid_price`, `ask_price`, `last_price` are filled when the provider publishes them.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aeon-market-scanner-rs = "0.9"
tokio = { version = "1", features = ["full"] }
```

Or pin the exact version:

```toml
aeon-market-scanner-rs = "0.7.1"
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

## Stocks (S&P 500) and fiat exchange rates

```rust,no_run
use aeon_market_scanner_rs::{Alpaca, FiatTrait, Frankfurter, StockTrait};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    // Fiat rates: no API key. Accepts "EUR/USD", "EURUSD", "eur-usd".
    let fx = Frankfurter::new();
    for rate in fx.get_rates(&["EUR/USD", "GBP/USD", "USD/TRY"]).await? {
        println!("{} = {} ({})", rate.pair, rate.rate, rate.date);
    }

    // US stocks: needs ALPACA_API_KEY_ID and ALPACA_API_SECRET_KEY.
    let _ = dotenvy::dotenv();
    let stocks = Alpaca::from_env()?;
    let aapl = stocks.get_price("AAPL").await?;
    println!("{} price={} bid={:?} ask={:?}", aapl.symbol, aapl.price, aapl.bid_price, aapl.ask_price);

    // All S&P 500 constituents, batched into a few requests.
    let sp500 = stocks.get_sp500_prices().await?;
    println!("{} quotes", sp500.len());

    Ok(())
}
```

- Keys are read from the environment by each provider's `from_env()`: `ALPACA_API_KEY_ID` + `ALPACA_API_SECRET_KEY`, `TIINGO_API_TOKEN`, `FINNHUB_API_KEY`.
- Free-plan caveats: Tiingo Starter allows 500 unique symbols per month (the S&P 500 has 503); Finnhub has no batch endpoint, so long lists hit its per-minute limit; `YahooStocks` / `YahooFiat` use an undocumented endpoint (see below).
- Alpaca uses the free IEX feed by default; call `.with_feed(AlpacaFeed::Sip)` if you have the paid subscription. `price` is the bid/ask midpoint when the quote is present and at most 2% wide (`.with_max_spread(..)`), otherwise the last trade, otherwise the daily close. Outside US market hours IEX often has no usable quote, so most tickers then come from the last trade, with `bid_price` / `ask_price` left empty.
- Frankfurter, ExchangeRate-API and TCMB serve daily reference rates, not live prices; `FiatRate.date` tells you which day they are from.
- For real-time rates use `TiingoFiat::from_env()?` (same `TIINGO_API_TOKEN` as the stock provider): `get_rates(&["EUR/USD", "USD/TRY"])` returns interbank bid/ask, usually seconds old, in one request. Pairs Tiingo lists the other way round (`TRY/USD`) are inverted automatically. The free plan's 50 requests/hour and 500 symbols/month are shared with Tiingo stock requests.
- Near-live fiat rates (usually under a minute old) and stock prices are available from Yahoo Finance behind an opt-in feature:

  ```toml
  aeon-market-scanner-rs = { version = "0.9", features = ["unofficial-yahoo"] }
  ```

  ```rust,ignore
  use aeon_market_scanner_rs::{FiatTrait, StockTrait, YahooFiat, YahooStocks};
  let rate = YahooFiat::new().get_rate("USD/TRY").await?; // rate.exchange_timestamp = time of the price
  let aapl = YahooStocks::new().get_price("AAPL").await?;
  ```

  This uses an **undocumented** Yahoo endpoint: Yahoo's terms do not allow automated access, and it can rate limit (HTTP 429), block or change at any time. It returns the last price only (no bid/ask). Use it for personal use and prototypes, and do not re-publish the data.
- Same `FiatTrait` API for every provider: `ExchangeRateApi::new()` covers 160+ currencies, `Tcmb::new()` adds `bid` / `ask` (TCMB buying/selling) for TRY pairs and crosses like `EUR/USD`.
- ExchangeRate-API caches each table for 1 hour and TCMB its bulletin for 15 minutes (`.with_cache_ttl(..)`, `Duration::ZERO` turns it off), since the data changes once per day.
- ExchangeRate-API's open access terms require attribution ("Rates By Exchange Rate API" linking to exchangerate-api.com), forbid redistributing the data, and rate-limit callers polling more often than about once per hour.

## MEXC: balances, orders, private order stream (signed API)

Public ticker data works with `Mexc::new()`. For **signed** REST and the **private** user order WebSocket, pass API keys explicitly — the crate does **not** read `MEXC_API_KEY` / `MEXC_API_SECRET` from the environment.

```rust,no_run
use aeon_market_scanner_rs::cex::mexc::Mexc;

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let mexc = Mexc::with_credentials("your_api_key", "your_api_secret");

    let snapshot = mexc.get_account_balances().await?;
    println!("balances: {}", snapshot.balances.len());

    let _order_updates = mexc.stream_spot_order_updates(5, 5_000).await?;
    // Spot trading: `place_market_order_*`, `place_limit_order`, `cancel_order`,
    // `place_batch_orders` / `place_batch_limit_orders` on `Mexc`.
    // `recv()` on the channel for private order fills (protobuf-backed on MEXC).

    Ok(())
}
```

### MEXC batch spot orders

Up to **20** orders per request, **same symbol** on every leg. Use `place_batch_limit_orders` for limit legs, or build `MexcBatchOrderItem` (e.g. `market_by_quantity`, `market_by_quote_amount`, `limit`) and call `place_batch_orders`.

```rust,no_run
use aeon_market_scanner_rs::cex::mexc::{
    Mexc, MexcBatchOrderItem, MexcLimitOrderType, MexcTradeSide,
};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let mexc = Mexc::with_credentials("your_api_key", "your_api_secret");
    let symbol = "BTCUSDT";

    // Limit batch: price + quantity per leg (here: two IOC orders — adjust prices to the book).
    let limit_rows = mexc
        .place_batch_limit_orders(symbol, &[
            (
                MexcTradeSide::Buy,
                "65000.0",
                "0.0001",
                MexcLimitOrderType::ImmediateOrCancel,
            ),
            (
                MexcTradeSide::Sell,
                "70000.0",
                "0.0001",
                MexcLimitOrderType::ImmediateOrCancel,
            ),
        ])
        .await?;
    for row in &limit_rows {
        println!("{row:?}");
    }

    // Market batch: base quantity per leg (same symbol).
    let market_batch = [
        MexcBatchOrderItem::market_by_quantity(symbol, MexcTradeSide::Buy, "0.0001")?,
        MexcBatchOrderItem::market_by_quantity(symbol, MexcTradeSide::Sell, "0.0001")?,
    ];
    let market_rows = mexc.place_batch_orders(&market_batch).await?;
    for row in &market_rows {
        println!("{row:?}");
    }

    Ok(())
}
```

See `Mexc` and `cex::mexc` re-exports for types such as `MexcSpotOrderUpdate`, `MexcPlacedOrder`, `MexcBatchOrderItem`, `MexcBatchOrderResult`, `MexcLimitOrderType`, `MexcTradeSide`.

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

## DEX pool price listener (Uniswap V2 / V3 / V4, PancakeSwap Infinity)

Stream live prices from multiple pools over a single WebSocket connection. Listens only to **Swap** events (`eth_subscribe("logs")`); price is computed from swap event parameters. No block subscription or separate RPC for reads; decimals and symbol come from the `PoolWithTokens` you pass.

**Pool kinds:** `V2`, `V3`, `V4` (Uniswap V4), `V4Pancake` (PancakeSwap Infinity). For V4 and V4Pancake you must set `pool_id` (bytes32 from the chain); `pool_address` is the PoolManager contract address.

```rust,no_run
use aeon_market_scanner_rs::{
    stream_pool_prices,
    PoolKind, PoolWithTokens, PriceDirection, Token,
    dex::chains::ChainId,
};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let _ = dotenvy::dotenv(); // or load .env in tests; add dotenvy to your Cargo.toml if needed
    let rpc_ws = std::env::var("POOL_LISTENER_RPC_WS").expect("POOL_LISTENER_RPC_WS");

    let token0 = Token::create("0x...", "BNB", "BNB", 18, ChainId::BSC);
    let token1 = Token::create("0x...", "USDT", "USDT", 18, ChainId::BSC);
    let pool = PoolWithTokens {
        pool_address: "0x16b9a82891338f9bA80E2D6970FddA79D1eb0daE".to_string(),
        pool_kind: PoolKind::V2Pancake,
        pool_id: None, // required for V4 / V4Pancake
        token0,
        token1,
        price_direction: PriceDirection::Token0PerToken1,
        fee_bps: 30, // e.g. 30 for V2, 500/3000/10000 for V3
    };

    // You can pass multiple pools here!
    let pools = vec![pool];
    let mut rx = stream_pool_prices(rpc_ws, ChainId::BSC, pools, 3, 5000).await?;
    while let Some(update) = rx.recv().await {
        println!("bid={} ask={} mid={} block={} symbol={:?}", update.bid, update.ask, update.mid, update.block_number, update.symbol);
    }
    Ok(())
}
```

- **PoolWithTokens**: `pool_address`, `pool_kind`, optional `pool_id` (for V4/V4Pancake), `token0`, `token1`, `price_direction`, `fee_bps`. Decimals and symbol are taken from the tokens; `fee_bps` is used for bid/ask spread (e.g. 30 for V2, 500/3000/10000 for V3).
- **PriceDirection**: `Token1PerToken0` or `Token0PerToken1`.
- **Reconnect**: `reconnect_attempts` = 0 to disable; n = up to n reconnects. `reconnect_delay_ms` = delay in ms.
- V2/V3: `pool_address` = pair/pool contract. V4/V4Pancake: `pool_address` = PoolManager, `pool_id` = topic1 from chain explorer.
- **V4 CL fee note**: For V4 CL pools, fee is read from the Swap event data, so `fee_bps` in `PoolWithTokens` is ignored. You can set `fee_bps: 0` for V4 CL configs.

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
use aeon_market_scanner_rs::{ArbitrageScanner, CexExchange, ScannerEvent};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let mut rx = ArbitrageScanner::scan_arbitrage_from_websockets(
        &["BTCUSDT", "ETHUSDT"],
        &[CexExchange::Binance, CexExchange::OKX, CexExchange::Bybit],
        None,
        None, // No DEX streams in this example
        None, // No DEX RPC WS URL
        None, // No chain ID for DEX
        10,   // reconnect_attempts
        5000, // reconnect_delay_ms
    )
    .await?;

    while let Some(event) = rx.recv().await {
        match event {
            ScannerEvent::Opportunity(opps) => {
                for o in opps.iter().take(5) {
                    println!(
                        "{} -> {} {} spread={:.4} ({:.3}%)",
                        o.source_exchange, o.destination_exchange, o.symbol,
                        o.spread, o.spread_percentage
                    );
                }
            }
            ScannerEvent::Tick => {
                // Heartbeat / snapshot marker
            }
            ScannerEvent::Price(price) => {
                // Individual price update (useful if you want to log raw stream events)
            }
        }
    }

    Ok(())
}
```

Exchanges that do not support WebSocket are skipped. The receiver emits `ScannerEvent::Opportunity` (sorted by profitability) whenever a snapshot evaluation completes, `ScannerEvent::Tick` for heartbeats, and `ScannerEvent::Price` for individual price updates.

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

- **Public APIs**: default CEX usage (`get_price`, public WebSockets, arbitrage scanner) uses **public** endpoints only — **no API keys** for those paths. **MEXC signed features** (`Mexc::with_credentials`, balances, trading, private order stream) require keys you supply in code. **Alpaca stock prices** (`stocks`) also require keys. Usage is subject to each provider’s rate limits and terms.
- **Network + rate limits**: exchange APIs can rate-limit or temporarily fail; callers should expect errors.
- **Symbols**: most examples use common `BASEQUOTE` format like `BTCUSDT`. Some exchanges may require different formatting internally; the crate normalizes per-exchange.
- **WebSocket streams**: intended for continuous feeds. When the receiver ends (`None`), the underlying connection has closed (and may reconnect if `reconnect_attempts` > 0).
- **Pool listener**: requires a WebSocket RPC URL (e.g. from Alchemy, Infura, or a chain node). Block delivery depends on the RPC; block numbers may not be consecutive.

## License

Licensed under the **Apache License, Version 2.0**. See `LICENSE`.
