# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-02-06

### Added

- Initial public release.
- CEX REST price fetching across supported exchanges.
- Optional WebSocket streaming price feeds with reconnect/backoff and `max_attempts`.
- Arbitrage scanner for CEX↔CEX and optional DEX legs (KyberSwap).

