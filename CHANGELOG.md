# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
- Public helper `ArbitrageScanner::opportunities_from_prices(...)` for deterministic/offline opportunity evaluation.
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
