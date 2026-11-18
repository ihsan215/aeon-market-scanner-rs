use serde::{Deserialize, Serialize};

/// Data providers for US stock prices.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum StockProvider {
    /// Alpaca Market Data API (bid/ask quotes).
    Alpaca,
    /// Finnhub `/quote` (last price).
    Finnhub,
    /// Tiingo IEX endpoint (last price; bid/ask only with an IEX data agreement).
    Tiingo,
    /// Yahoo Finance, unofficial endpoint (feature `unofficial-yahoo`).
    Yahoo,
}

/// Price of a US stock.
///
/// Providers differ in what they publish: some give a bid/ask quote, others only the last
/// trade. [`price`](Self::price) is always set; the other price fields are filled when the
/// provider has them.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StockPrice {
    /// Ticker in standard form, e.g. `AAPL`, `BRK.B`.
    pub symbol: String,
    /// Best single price: the bid/ask midpoint when the provider gives a quote, otherwise the
    /// last trade price.
    pub price: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_price: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_price: Option<f64>,
    /// Bid size in shares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bid_qty: Option<f64>,
    /// Ask size in shares.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ask_qty: Option<f64>,
    /// Last trade price, for providers that report one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_price: Option<f64>,
    /// Local receive time (ms since epoch): when this library parsed the response.
    pub timestamp: u64,
    /// Time of the quote or trade as reported by the provider (ms since epoch). Outside market
    /// hours this can be hours old, so compare it with `timestamp` before treating the price
    /// as live.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exchange_timestamp: Option<u64>,
    pub provider: StockProvider,
}
