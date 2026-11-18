# aeon-market-scanner-rs

A Rust crate for fetching **CEX** and **DEX** prices and finding **arbitrage opportunities**.

- REST price fetching (`get_price`)
- CEX WebSocket streams (`stream_price_websocket`) with configurable reconnect (attempts + delay in ms)
- **MEXC signed API** (optional): account balances, spot orders (market/limit/cancel, **batch** market/limit), and a **private** spot order WebSocket stream — use `Mexc::with_credentials(...)` (keys are **not** read from env inside the crate)
- **DEX pool price listener**: Uniswap V2, V3, V4 and PancakeSwap Infinity (V4-style) pool prices over WebSocket RPC (`stream_pool_prices`); swap-event only, price from event params
- Arbitrage scanning: one-shot REST (`scan_arbitrage_opportunities`) or live WebSocket (`scan_arbitrage_from_websockets` with `ScannerEvent` stream)
- Fee overrides (VIP/custom tiers) and optional DEX legs (KyberSwap)
- **US stocks** (S&P 500) and **fiat exchange rates** from several providers (Alpaca, Tiingo, Finnhub, Frankfurter, ExchangeRate-API, TCMB, Tiingo FX, optional Yahoo), with **fallback chains** that prefer live data and switch provider on errors

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

### Stocks and fiat (separate modules)

| Module   | Provider                     | Key  | Live?             | Bid/ask | Batch                | Free-plan limits                          |
| -------- | ---------------------------- | ---- | ----------------- | ------- | -------------------- | ----------------------------------------- |
| `stocks` | Alpaca                       | yes  | yes (IEX)         | yes     | 200 tickers/request  | 200 requests/min                          |
| `stocks` | Tiingo                       | yes  | yes               | no¹     | 100 tickers/request  | 50 req/hour, 1,000/day, 500 symbols/month |
| `stocks` | Finnhub                      | yes  | yes               | no      | no (1 per ticker)    | 60 requests/min                           |
| `stocks` | Yahoo (opt-in, unofficial)   | no   | yes               | no      | no (1 per ticker)    | undocumented, may block                   |
| `fiat`   | Tiingo FX                    | yes  | **yes (seconds)** | yes     | 100 pairs/request    | shares the Tiingo quota above             |
| `fiat`   | Yahoo (opt-in, unofficial)   | no   | ~1 minute         | no      | no (1 per pair)      | undocumented, may block                   |
| `fiat`   | ExchangeRate-API             | no   | daily             | no      | per base currency    | ~1 request/hour, attribution required     |
| `fiat`   | Frankfurter (ECB)            | no   | daily             | no      | per base currency    | –                                         |
| `fiat`   | TCMB (Turkish central bank)  | no   | daily             | yes     | whole bulletin       | –                                         |

¹ Tiingo returns bid/ask only to accounts with an IEX market data agreement.

See [Stocks and fiat exchange rates](#stocks-and-fiat-exchange-rates) for usage.

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
aeon-market-scanner-rs = "0.11"
tokio = { version = "1", features = ["full"] }
```

Or pin the exact version:

```toml
aeon-market-scanner-rs = "0.11.0"
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

## Stocks and fiat exchange rates

US stock prices live in the `stocks` module and fiat exchange rates in `fiat`. They are separate from `cex` / `dex`, have their own result types (`StockPrice`, `FiatRate`), and are not part of the arbitrage scanner.

Every provider implements one trait per module, so they are interchangeable:

| Trait        | Methods                                                  |
| ------------ | -------------------------------------------------------- |
| `StockTrait` | `get_price(symbol)`, `get_prices(&[..])`, `get_sp500_prices()`, `health_check()` |
| `FiatTrait`  | `get_rate(pair)`, `get_rates(&[..])`, `health_check()`   |

Import the trait together with the provider, otherwise the methods are not in scope.

> Available from version 0.10.0.

### 1. API keys

Providers that need a key read it from the environment in `from_env()` (a `.env` file works with `dotenvy`). The fiat providers Frankfurter, ExchangeRate-API and TCMB need no key.

```env
ALPACA_API_KEY_ID=...       # alpaca.markets → paper account → API Keys
ALPACA_API_SECRET_KEY=...
TIINGO_API_TOKEN=...        # tiingo.com → Account → API → Token (stocks and Tiingo FX)
FINNHUB_API_KEY=...         # finnhub.io → Dashboard
```

Every key is optional; set the ones for the providers you use. All of them have a free plan.

### 2. Recommended: fallback chains

`StockFallback` and `FiatFallback` ask several providers in order and give you, for each symbol, the first **live** price. Live means `exchange_timestamp` is at most 15 minutes old by default. They implement `StockTrait` / `FiatTrait`, so you call them like any single provider.

```rust,no_run
use aeon_market_scanner_rs::{FiatFallback, FiatTrait, StockFallback, StockTrait};

#[tokio::main]
async fn main() -> Result<(), aeon_market_scanner_rs::MarketScannerError> {
    let _ = dotenvy::dotenv();

    // Every stock provider whose key is set: Alpaca → Tiingo → Finnhub (→ Yahoo with the feature).
    let stocks = StockFallback::from_env()?;
    let aapl = stocks.get_price("AAPL").await?;                       // one ticker
    let some = stocks.get_prices(&["MSFT", "NVDA", "BRK.B"]).await?; // several
    let sp500 = stocks.get_sp500_prices().await?;                     // all 503 S&P 500 tickers
    println!("AAPL {} from {:?}; {} + {} prices", aapl.price, aapl.provider, some.len(), sp500.len());

    // Tiingo FX (if keyed) → Yahoo (feature) → ExchangeRate-API → Frankfurter → TCMB.
    let fx = FiatFallback::from_env();
    let usd_try = fx.get_rate("USD/TRY").await?;
    println!("USD/TRY {} bid={:?} ask={:?} from {:?}", usd_try.rate, usd_try.bid, usd_try.ask, usd_try.provider);
    for r in fx.get_rates(&["EUR/USD", "GBP/JPY", "USD/ARS"]).await? {
        println!("{} {} from {:?}", r.pair, r.rate, r.provider); // USD/ARS is not on Tiingo FX: it falls back
    }
    Ok(())
}
```

Or choose the order yourself:

```rust,ignore
use aeon_market_scanner_rs::{Alpaca, ExchangeRateApi, FiatFallback, Finnhub, StockFallback, Tcmb, TiingoFiat};

let stocks = StockFallback::new(Alpaca::from_env()?).then(Finnhub::from_env()?);
let fx = FiatFallback::new(TiingoFiat::from_env()?).then(Tcmb::new()).then(ExchangeRateApi::new());
```

How a chain decides, per symbol:

1. Ask the first provider.
2. If it **errors** (rate limit, network, auth), **does not know** the symbol, or its price is **older than the age limit**, ask the next provider. Only the symbols still missing are sent on.
3. Return the first live price found. If no provider has one, return the first older price found in provider order, so you still get the last known value.
4. `result.provider` says which provider answered; `result.exchange_timestamp` (ms since epoch) says how old the price is.

When a symbol is served by no provider, `get_price` / `get_rate` return an error. `StockFallback::get_prices` leaves that ticker out, and `FiatFallback::get_rates` returns an error naming the missing pairs.

Options:

| Option                                   | Default    | Effect                                                                                                    |
| ---------------------------------------- | ---------- | --------------------------------------------------------------------------------------------------------- |
| `.with_max_age(Some(duration))`          | 15 minutes | Oldest price still treated as live.                                                                       |
| `.with_max_age(None)`                    |            | No age check: move on only on errors and unknown symbols.                                                 |
| `.with_age_limit_in_session_only(false)` | `true`     | Stocks only. By default the age check runs only during the US session (Mon–Fri 09:30–16:00 New York). Outside it every provider's price is old, so asking more providers would only spend their quota. |

Asking more providers costs their quota. Put batch-capable providers first: Alpaca and Tiingo for stocks, Tiingo FX for fiat. Tiingo's free plan allows 50 requests/hour and 500 symbols/month, shared between stocks and FX, so do not make it the backup for the whole S&P 500.

### 3. Using a single provider

```rust,ignore
use aeon_market_scanner_rs::{Alpaca, FiatTrait, StockTrait, Tcmb, TiingoFiat};

let nvda = Alpaca::from_env()?.get_price("NVDA").await?;
let live = TiingoFiat::from_env()?.get_rates(&["EUR/USD", "USD/TRY"]).await?; // real-time bid/ask
let tcmb = Tcmb::new().get_rate("USD/TRY").await?;                             // official daily rate
```

Symbols are normalised: `brk-b`, `BRK/B` and `BRK.B` are the same ticker, and `EUR/USD`, `EURUSD`, `eur-usd` and `EUR_USD` are the same pair.

### 4. What you get back

`StockPrice`:

| Field                                         | Meaning                                                                         |
| --------------------------------------------- | ------------------------------------------------------------------------------- |
| `symbol`                                      | Ticker in standard form (`AAPL`, `BRK.B`)                                        |
| `price`                                       | Always set: bid/ask midpoint when there is a usable quote, otherwise the last trade, otherwise the daily close |
| `bid_price`, `ask_price`, `bid_qty`, `ask_qty` | Filled only when the provider has a usable quote (not crossed, spread ≤ 2%)     |
| `last_price`                                  | Last trade price, when reported                                                 |
| `exchange_timestamp`                          | Time of that quote or trade (ms since epoch)                                     |
| `timestamp`                                   | When this crate received it                                                     |
| `provider`                                    | `Alpaca`, `Tiingo`, `Finnhub` or `Yahoo`                                         |

`FiatRate`:

| Field                | Meaning                                                                         |
| -------------------- | ------------------------------------------------------------------------------- |
| `pair`, `base`, `quote` | `EUR/USD`, `EUR`, `USD`; `1 base = rate quote`                                |
| `rate`               | Mid rate                                                                        |
| `bid`, `ask`         | Filled by Tiingo FX (interbank quote) and TCMB (buying/selling)                 |
| `date`               | Day the rate belongs to (`YYYY-MM-DD`)                                          |
| `exchange_timestamp` | Quote / publication time (ms since epoch), when the provider reports one        |
| `provider`           | `Tiingo`, `Yahoo`, `ExchangeRateApi`, `Frankfurter` or `Tcmb`                   |

### 5. Provider notes

- **Alpaca**: snapshots with quote, last trade and daily close in one call; 200 tickers per request, so the S&P 500 takes 3 requests. The free IEX feed often has no usable quote outside US market hours, so prices then come from the last IEX trade. That can differ slightly from the official close. `.with_feed(AlpacaFeed::Sip)` selects the paid all-exchange feed; `.with_max_spread(0.05)` accepts wider quotes.
- **Tiingo**: official last price, 100 tickers per request. The Starter plan's 500 symbols/month is less than the S&P 500's 503 tickers.
- **Finnhub**: official last price; one request per ticker, so it is best for short lists.
- **Tiingo FX**: interbank bid/ask, usually seconds old, for about 140 pairs in one request. Pairs listed only the other way round (`TRY/USD` from `USD/TRY`) are inverted automatically.
- **ExchangeRate-API**: 165 currencies, daily. Polling more often than about once per hour gets your IP blocked (HTTP 429) for 20 minutes. Its terms require the attribution "Rates By Exchange Rate API" linking to exchangerate-api.com, and forbid redistributing the data.
- **Frankfurter**: ECB reference rates for about 30 currencies, updated on working days around 16:00 CET.
- **TCMB**: the Turkish central bank's daily indicative rates for about 20 currencies against TRY, with buying/selling prices. Crosses such as `EUR/USD` are computed through TRY.
- **Yahoo** (`YahooStocks`, `YahooFiat`): no key and near-live, but it uses an **undocumented** endpoint that Yahoo's terms do not permit for automated access, and it can rate limit or break without notice. It is therefore behind an opt-in feature:

  ```toml
  aeon-market-scanner-rs = { version = "0.11", features = ["unofficial-yahoo"] }
  ```

> **Caching:** the crate does not cache. Every call goes to the provider. Daily sources (ExchangeRate-API, Frankfurter, TCMB) change once per day and some rate-limit heavy polling, so keep the results on your side (in memory, Redis, a database) and refresh them hourly or so.

> **Licensing:** free plans are for personal or internal use. Showing their data on a public website counts as redistribution and usually needs a commercial plan, and for stocks often exchange display licences. See [Disclaimer](#disclaimer) for each provider's terms.

### 6. Try it

```sh
cargo run --example fallback                            # stocks + fiat through the fallback chains
cargo run --example fallback --features unofficial-yahoo
cargo test --test fallback_test -- --nocapture          # live tests; providers without keys are skipped
```

### Adding a provider

Create `src/stocks/<provider>/` or `src/fiat/<provider>/`, add a variant to `StockProvider` / `FiatProvider`, and implement `StockTrait` / `FiatTrait`. Override `get_prices` / `get_rates` if the API has a batch endpoint. The new provider then works on its own and inside the fallback chains.

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

- **Public APIs**: default CEX usage (`get_price`, public WebSockets, arbitrage scanner) uses **public** endpoints only — **no API keys** for those paths. **MEXC signed features** (`Mexc::with_credentials`, balances, trading, private order stream) require keys you supply in code. Stock providers (Alpaca, Tiingo, Finnhub) and Tiingo FX also need keys, read from the environment by `from_env()`. Usage is subject to each provider’s rate limits and terms.
- **Network + rate limits**: exchange APIs can rate-limit or temporarily fail; callers should expect errors.
- **Symbols**: most examples use common `BASEQUOTE` format like `BTCUSDT`. Some exchanges may require different formatting internally; the crate normalizes per-exchange.
- **WebSocket streams**: intended for continuous feeds. When the receiver ends (`None`), the underlying connection has closed (and may reconnect if `reconnect_attempts` > 0).
- **Pool listener**: requires a WebSocket RPC URL (e.g. from Alchemy, Infura, or a chain node). Block delivery depends on the RPC; block numbers may not be consecutive.

## Disclaimer

- **Not affiliated.** This project is not affiliated with, endorsed by or sponsored by any exchange or data provider it connects to. Exchange and provider names are trademarks of their respective owners and are used only to say which API a module talks to.
- **No data is distributed by this crate.** It is client code: every request goes from your machine to the provider, under your own account or API key where one is needed. You are bound by each provider's terms.
- **Your use of the data is your responsibility.** Most providers allow personal or internal use but **prohibit redistribution or public display** without a separate written licence. Examples are showing prices on a public website, re-serving them through your own API, or posting them to public channels. Crediting the source does not replace that licence. Check the terms of every provider you use:

  | Provider | Terms |
  | --- | --- |
  | Binance | [binance.com/en/terms](https://www.binance.com/en/terms) |
  | Coinbase | [Market Data Terms of Use](https://www.coinbase.com/legal/market_data) |
  | Kraken | [kraken.com/legal](https://www.kraken.com/legal) |
  | OKX | [OKX API Agreement](https://www.okx.com/help/okx-api-agreement) |
  | Bybit | [API Terms & Conditions](https://www.bybit.com/en/help-center/article/API-Terms) |
  | Other exchanges (MEXC, Gate, KuCoin, Bitget, BtcTurk, HTX, Bitfinex, Upbit, Crypto.com) | the API / market data terms on each exchange's website |
  | KyberSwap | the KyberSwap aggregator API terms |
  | Alpaca | [alpaca.markets/disclosures](https://alpaca.markets/disclosures) |
  | Tiingo (stocks and FX) | [api.tiingo.com/documentation](https://api.tiingo.com/documentation) (usage and licensing section) |
  | Finnhub | [finnhub.io/terms-of-service](https://finnhub.io/terms-of-service) |
  | ExchangeRate-API | [exchangerate-api.com/terms](https://www.exchangerate-api.com/terms) (attribution required, no redistribution) |
  | Frankfurter / ECB | [frankfurter.dev](https://frankfurter.dev) · [ECB legal notice](https://www.ecb.europa.eu/home/disclaimer/html/index.en.html) (reuse allowed with the source credited) |
  | TCMB | [Kullanım Koşulları](https://www.tcmb.gov.tr/wps/wcm/connect/ekonomi/hie/diger/kullanim+kosullari) (commercial use needs written permission) |
  | Yahoo Finance (`unofficial-yahoo`) | [Yahoo Terms](https://legal.yahoo.com/us/en/yahoo/terms/otos/index.html). The endpoint is undocumented and automated access is not permitted, so use it at your own risk. |

- **No warranty; not financial advice.** Prices can be delayed, incomplete, stale or wrong. Do not rely on them for trading without your own checks. The software is provided "as is" under the Apache License 2.0, without warranties or liability. Trading and arbitrage involve risk of loss; signed MEXC features place real orders with real funds.

## License

Licensed under the **Apache License, Version 2.0**. See `LICENSE`.
