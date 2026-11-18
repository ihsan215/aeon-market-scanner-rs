# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0] - 2026-11-18

### Added

- New modules for stocks and fiat currencies, separate from `cex` and `dex`. Each provider has its own folder and implements a shared trait, so more providers can be added later.
- **`stocks` module: US stock prices.**
  - The `StockTrait` trait defines `get_price`, `get_prices` and `get_sp500_prices`.
    - Every provider's `get_prices` returns one entry per distinct ticker, in input order.
    - It drops malformed and unknown tickers instead of failing the batch, and returns other errors, such as a rate limit.
    - Alpaca splits a rejected batch to isolate the bad ticker.
    - The default implementation keeps 5 requests in flight.
  - Each result is a `StockPrice`:
    - `price` is always set. It is the bid/ask midpoint when the provider has a quote, otherwise the last trade.
    - `bid_price`, `ask_price`, `bid_qty`, `ask_qty` and `last_price` are optional.
    - `exchange_timestamp` is the quote or trade time.
  - Providers:
    - `Alpaca` uses `/v2/stocks/snapshots` on the free IEX feed.
      - Credentials: `ALPACA_API_KEY_ID` and `ALPACA_API_SECRET_KEY`.
      - It sends up to 200 tickers per request.
      - `price` is the bid/ask midpoint when the quote is not crossed and at most 2% wide (`with_max_spread`, default `stocks::DEFAULT_MAX_SPREAD`). Otherwise it is the latest trade, and failing that the daily close.
      - A daily-close fallback is stamped with the 16:00 New York session close, not the bar's midnight start.
      - So all S&P 500 tickers get a price even when the market is closed.
      - `AlpacaFeed` selects the feed: `Iex`, `Sip` or `DelayedSip`.
    - `Tiingo` uses the IEX endpoint (`TIINGO_API_TOKEN`).
      - It sends up to 100 tickers per request.
      - It returns the last price. Bid/ask come only with an IEX data agreement, and then pass the same crossed and spread check (`with_max_spread`).
    - `Finnhub` uses `/quote` (`FINNHUB_API_KEY`). It returns the last price, one request per ticker.
    - `YahooStocks` returns the last price. It needs no key and sits behind the opt-in `unofficial-yahoo` feature.
  - `SP500_SYMBOLS` lists the 503 S&P 500 constituents as of 2026-11-18, taken from Wikipedia.
  - `normalize_stock_symbol` helper: `brk-b` → `BRK.B`.
- **`fiat` module: fiat exchange rates.**
  - The `FiatTrait` trait defines `get_rate` and `get_rates`.
  - `fiat::frankfurter::Frankfurter` is the first provider. It serves ECB reference rates and needs no API key.
    - `get_rates` sends one request per base currency and keeps input order.
    - It also has `get_all_rates(base)` and `supported_currencies()`.
  - Each result is a `FiatRate`. Its `bid`, `ask` and `exchange_timestamp` fields are optional and filled when the provider publishes them.
  - `fiat::exchangerate_api::ExchangeRateApi` serves the open access endpoint of ExchangeRate-API. It needs no API key.
    - It covers 160+ currencies and updates daily.
    - It has `get_all_rates(base)`.
    - Its terms require attribution and forbid redistribution.
    - It caches each table for 1 hour (`with_cache_ttl`), so it stays within the provider's hourly polling guidance.
    - An unknown currency maps to `InvalidSymbol` whatever the HTTP status, and HTTP 429 gets a clear rate-limit error.
  - `fiat::tcmb::Tcmb` serves the indicative rates of the Central Bank of the Republic of Türkiye. It needs no API key.
    - It covers about 20 currencies against TRY, with forex buying/selling prices returned as `bid` / `ask`.
    - Cross pairs such as `EUR/USD` are computed through TRY.
    - JPY is normalised from its 100-unit quote.
    - It has `get_all_rates()`.
    - It caches the bulletin for 15 minutes (`with_cache_ttl`).
  - `fiat::tiingo::TiingoFiat` serves Tiingo FX real-time top-of-book quotes, using the same `TIINGO_API_TOKEN` as the stock provider.
    - It covers about 140 pairs, with bid/ask and the quote time.
    - It fetches many pairs per request.
    - Pairs listed only the other way round (`TRY/USD`) are fetched and inverted.
  - `fiat::yahoo::YahooFiat` serves near-live Yahoo Finance rates for practically all pairs. It sits behind the opt-in **`unofficial-yahoo`** Cargo feature, which is off by default.
    - It uses an undocumented endpoint that Yahoo's terms do not permit for automated access, so it may rate limit or break.
    - It returns the last price only; `exchange_timestamp` is the time of that price.
    - It keeps at most 4 requests in flight.
    - Any "not found" answer (404, or a chart error) is treated as an unknown symbol, so one odd ticker does not abort a batch.
  - docs.rs builds with all features, so the opt-in Yahoo providers are documented.
  - `parse_fiat_pair` and `parse_currency_code` helpers: `EUR/USD`, `EURUSD` and `eur-usd` all parse to `(EUR, USD)`.
- **Fallback chains: `StockFallback` and `FiatFallback`.**
  - They try providers in order and return, per symbol, the first live price. Live means `exchange_timestamp` is within `with_max_age`, 15 minutes by default.
  - On an error, an unknown symbol or a stale price, the next provider is asked, and only for the symbols still missing. If no provider has a live price, the first stale one is returned.
  - Both implement `StockTrait` / `FiatTrait`, so single and batch calls work.
  - Build a chain with `new(..).then(..)`, or with `from_env()` to use every provider that has credentials.
  - Stock chains apply the age limit only during the US regular session, adjusted for DST (`with_age_limit_in_session_only`). This avoids spending fallback quotas while the market is closed.
  - `FiatFallback` retries a provider's pairs one by one when its batch call fails, so one uncovered pair does not hide the others.
  - `examples/fallback.rs` shows both chains: `cargo run --example fallback`.
- Tests:
  - Unit tests for parsing.
  - Live tests in `tests/fiat_frankfurter_test.rs`, `tests/fiat_exchangerate_api_test.rs`, `tests/fiat_tcmb_test.rs`, `tests/fiat_tiingo_test.rs`, `tests/fiat_yahoo_test.rs` and `tests/stocks_yahoo_test.rs`. The Yahoo tests run only with `--features unofficial-yahoo`.
  - `tests/stocks_alpaca_test.rs`, `tests/stocks_finnhub_tiingo_test.rs` and `tests/fallback_test.rs`. Each test is skipped when its provider's key is not set.

## [0.9.1] - 2026-11-12

### Changed

- Documentation only, no code changes:
  - `homepage` now points to [aeonfeed.com](https://aeonfeed.com), the free live demo and API built on this crate.
  - The README has a new "Live demo" section.

## [0.9.0] - 2026-11-12

### Added

- **`CexPrice.exchange_timestamp: Option<u64>`**: the event time reported by the exchange itself, in ms since epoch. It is filled for WebSocket prices from 9 venues:

  | Venue | Source field |
  |---|---|
  | Bybit | `cts`, falling back to `ts` |
  | OKX | `ts` |
  | KuCoin | `timestamp` |
  | Bitget | `ts` |
  | Coinbase | `time`, RFC 3339 |
  | Kraken | `timestamp`, RFC 3339 |
  | Upbit | `timestamp` |
  | Crypto.com | `t` |
  | MEXC | `createTime`, falling back to `sendTime` |

  - Compare it with `timestamp` (local receive time) to measure exchange → client latency or to detect lagging feeds, e.g. before acting on a cross-exchange spread.
  - It is `None` for feeds without an event time (Binance bookTicker, Bitfinex ticker, Gate v3 depth, BtcTurk, HTX) and for REST prices.
  - It is omitted from JSON when `None`, and old payloads without it still deserialize.
- `common::utils::json_ms()` and `common::utils::iso8601_ms()` helpers for parsing exchange timestamps.
- Tests:
  - Unit tests for timestamp parsing (number/string, s/ms/µs/ns, RFC 3339).
  - Live tests in `tests/exchange_timestamp_test.rs`.

### Changed (breaking)

- `CexPrice` has a new public field. Code that builds `CexPrice` with a struct literal must add `exchange_timestamp: None` (or a value). Code that only reads prices is unaffected.
- `CexPrice.timestamp` is unchanged and is now documented as the local receive time.

## [0.8.0] - 2026-11-12

### Fixed

- **Gate WebSocket prices were price-merged**: `depth.subscribe` used interval `"0.01"`, which rounds the book to 0.01 steps. Low-priced assets came back wrong (DOGE ~0.067 reported as bid 0.06 / ask 0.07). The interval is now `"0"` (no merging).
- **Bitfinex WebSocket symbols**: prices carried the raw channel name (`TBTCUST`, `TDOGE:UST`). They are now normalised to the standard `BTCUSDT` / `DOGEUSDT` form.
- **Symbols with 1–2 letter bases** (`OPUSDT`, `ARUSDT`, `ZKUSDT`, `SUSDT`, …) were formatted wrongly, or rejected, on venues that use a separator:
  - Affected venues: OKX, KuCoin, Coinbase, Gate, Crypto.com, Upbit, and Kraken WS. For example, OKX received `OPU-SDT` and Gate received `OPU_SDT`.
  - The quote-suffix checks now only require a non-empty base.

### Changed (breaking)

- **Bitfinex `CexPrice.symbol` is now the standard symbol** (`BTCUSDT`) for both REST and WebSocket. It was `BTCUST`.
  - Requests still use Bitfinex's `tBTCUST` naming internally.
  - Bitfinex prices now match other exchanges by symbol, e.g. in `ArbitrageScanner`. Before, they never matched `BTCUSDT`.
  - If you matched on `"BTCUST"`, switch to `"BTCUSDT"`.

### Added

- `common::utils::bitfinex_symbol_to_standard()` helper (`tBTCUST` / `tDOGE:UST` → `BTCUSDT` / `DOGEUSDT`).
- Unit tests for symbol formatting and Bitfinex/Upbit WS symbol normalisation. Unit test for the Gate subscribe message.
- Live regression tests:
  - `tests/short_base_symbols_test.rs`
  - `bitfinex_ws_reports_standard_symbols`
  - `gateio_ws_low_priced_assets_are_not_rounded`

## [0.7.1] - 2026-04-11

### Added

- **MEXC spot batch orders** ([`POST /api/v3/batchOrders`](https://www.mexc.com/api-docs/spot-v3/spot-account-trade#batch-orders)):
  - `place_batch_orders(&[MexcBatchOrderItem])`, `place_batch_limit_orders(...)` (same-symbol legs, max 20 per request).
  - Types: `MexcBatchOrderItem` (limit / market-by-qty / market-by-quote helpers), `MexcBatchOrderResult`.
- **Dependency**: `url` — signed REST query strings use form-urlencoded encoding (required for `batchOrders` JSON in the query).

### Changed

- **MEXC `signed_request`**: query parameters are now URL-encoded when building the string to sign and send (backward compatible for simple alphanumeric params).

### Notes

- Optional integration tests: `tests/mexc_batch_market_test.rs`, `tests/mexc_batch_limit_test.rs` (live API when `MEXC_API_KEY` / `MEXC_API_SECRET` are set).

## [0.7.0] - 2026-04-03

### Added

- **MEXC account & trading** (spot):
  - `get_account_balances()` — signed REST account snapshot.
  - `stream_spot_order_updates(...)` — private user order stream (`spot@private.orders.v3.api.pb`), protobuf-decoded pushes; receiver is returned only after WebSocket subscribe succeeds (reduces missed early fills).
  - `place_market_order_by_quantity` / `place_market_order_by_quote_amount`, `place_limit_order` (Limit / Post-only / IOC / FOK), `cancel_order`.
- Re-exported MEXC order/balance types (e.g. `MexcSpotOrderUpdate`, `MexcPlacedOrder`, `MexcLimitOrderType`, `MexcTradeSide`, …).

### Changed

- **Breaking (`Mexc`)**: struct is no longer generated only by `create_exchange!`. **`Mexc::new()`** — public market data only (no credentials). **`Mexc::with_credentials(api_key, api_secret)`** — required for signed REST and the private order WebSocket;
- **Dependencies**: `hmac`, `sha2`, `hex` for MEXC HMAC-SHA256 signing.

### Notes

- Integration tests under `tests/` (`mexc_*`) exercise live APIs when `MEXC_API_KEY` / `MEXC_API_SECRET` are set; they are optional for crate consumers.

## [0.6.0] - 2026-03-13

### Added

- DEX pool listener now distinguishes protocol-specific pool kinds (Uniswap/Pancake variants for V2/V3, plus V4 CL variants).
- Added chain gas estimate utility: `estimated_gas_cost_usd(ChainId) -> f64` with BSC set to 1 cent by default.
- Added extra pool listener tests for Uniswap V2 and Uniswap V3 scenarios.

### Changed

- Pool listener `stream_pool_prices` now accepts `chain_id: ChainId` (enum) instead of raw `u64`.
- `DexPrice.chain_id` now uses `ChainId` to keep chain typing consistent across listener/scanner outputs.
- V3 bid/ask calculation switched to two-direction `calc_amount_out` path with integer-safe U256 math.
- V4 bid/ask calculation now follows CL-style `calc_amount_out` and reads fee directly from V4 swap event data instead of external config.
- Scanner pool source naming now reflects enum chain id formatting (e.g. `Pool:BSC:...`).

## [0.5.0] - 2026-03-09

### Added

- **Multi-pool listener**: `stream_pool_prices` now accepts a `Vec<PoolWithTokens>` to stream multiple pools over a single WebSocket connection.
- **ScannerEvent Stream**: Scanner methods like `scan_arbitrage_from_websockets` now yield a `ScannerEvent` stream (`Tick`, `Price`, `Opportunity`).
- **DEX/CEX Arbitrage**: The scanner now fully supports live arbitrage scanning between CEXs and WebSocket-streamed DEX pools.
- **Modularity**: Separated `OpportunityFinder` logic into `opportunity.rs`.
- **DEX pool listener**: Uniswap V4 (`PoolKind::V4`) and PancakeSwap Infinity (`PoolKind::V4Pancake`) support. V4-style pools use PoolManager address + `pool_id` (bytes32); price from swap event data.
- **PoolWithTokens**: pool config struct with `pool_address`, `pool_kind`, optional `pool_id` (for V4/V4Pancake), `token0`, `token1`, `price_direction`. Decimals and symbol come from tokens; no on-chain decimals fetch.

### Changed

- **Breaking**: `DexPrice` has been renamed to `AggregatorPrice` and `DexRouteSummary` to `AggregatorRouteSummary` for REST aggregator requests (e.g. KyberSwap).
- **Breaking**: The `DexPrice` struct is now used to represent price updates originating from the live DEX pool WebSocket stream.
- **Breaking**: `stream_pool_prices` signature changed from taking a single `PoolListenerConfig` to taking `(rpc_ws_url, chain_id, pools: Vec<PoolWithTokens>, reconnect_attempts, reconnect_delay_ms)`.
- `dotenvy` is now included as a dependency for easier environment variable loading.
- **DEX pool listener (breaking)**: Swap-event only. No block subscription or `ListenMode`; no `symbol` or `public_rpc_urls`. `PoolListenerConfig` now takes a single `pool: PoolWithTokens` instead of `pool_address`, `pool_kind`, `listen_mode`, `symbol`, `public_rpc_urls`. Price is computed only from swap log parameters (V2: amount0/1 in/out; V3/V4: sqrtPriceX96). V3/V4 amounts use `I256` and safe `sqrtPriceX96` handling to avoid overflows.

### Removed

- **DEX pool listener**: `ListenMode`, `symbol`, from config. Block-based listening and read-only RPC fallback removed.

## [0.4.0] - 2026-02-06

### Added

- **DEX pool price listener**: stream live prices from Uniswap V2 / V3 pools over WebSocket RPC (`stream_pool_prices`, `PoolListenerConfig`). Supports `ListenMode::EveryBlock` or `OnSwapEvent`, `PriceDirection` (token1/token0 or token0/token1), optional reserves and `sqrt_price_x96`, and configurable reconnect (`reconnect_attempts`, `reconnect_delay_ms`).

### Changed

- **CEX WebSocket reconnect (breaking)**: unified with pool listener. `stream_price_websocket(symbols, reconnect_attempts, reconnect_delay_ms)` replaces `(symbols, reconnect, max_attempts)`. `reconnect_attempts`: 0 = no reconnect, n = up to n reconnects; `reconnect_delay_ms`: delay in milliseconds (0 → 1000 ms). Fixed delay between attempts (no exponential backoff).
- **Scanner**: `scan_arbitrage_from_websockets(..., reconnect_attempts, reconnect_delay_ms)` instead of `(..., reconnect, max_attempts)`.

## [0.3.1] - 2026-02-06

### Changed

- README: installation section and crate links updated for 0.3.x.

## [0.3.0] - 2026-02-06

### Added

- `ArbitrageScanner::scan_arbitrage_from_websockets(...)` – connect to CEX WebSocket streams and continuously receive arbitrage opportunity snapshots.

## [0.2.0] - 2026-02-06

### Added

- Fee override support via `FeeOverrides` (VIP/custom tiers) for arbitrage calculations.
- Public `ArbitrageScanner::find_opportunities(...)` for deterministic/offline opportunity evaluation.
- Additional public re-exports for fee helpers at crate root (e.g. `FeeOverrides`, `fee_rate`, `taker_fee_rate`).

### Changed

- `ArbitrageScanner::scan_arbitrage_opportunities(...)` now accepts an additional `fee_overrides: Option<&FeeOverrides>` parameter.
- WebSocket tests and README updated to match current public APIs and options.

## [0.1.0] - 2026-02-06

### Added

- Initial public release.
- CEX REST price fetching across supported exchanges.
- Optional WebSocket streaming price feeds with reconnect/backoff and `max_attempts`.
- Arbitrage scanner for CEX↔CEX and optional DEX legs (KyberSwap).
