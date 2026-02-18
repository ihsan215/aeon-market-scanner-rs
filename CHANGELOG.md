# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
