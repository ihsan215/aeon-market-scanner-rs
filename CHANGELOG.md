# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
