mod types;

use async_trait::async_trait;

use crate::common::utils::iso8601_ms;
use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::stocks::alpaca::types::{AlpacaSnapshot, AlpacaSnapshotsResponse};
use crate::stocks::{
    DEFAULT_MAX_SPREAD, StockPrice, StockProvider, StockTrait, credential_from_env,
    normalize_stock_symbol, normalize_stock_symbols, usable_quote_mid,
};

const ALPACA_DATA_API_BASE: &str = "https://data.alpaca.markets/v2";

/// Symbols sent per `/stocks/snapshots` request. The S&P 500 fits in 3 requests.
const MAX_SYMBOLS_PER_REQUEST: usize = 200;

/// Environment variable read by [`Alpaca::from_env`] for the API key id.
pub const ALPACA_KEY_ID_ENV: &str = "ALPACA_API_KEY_ID";
/// Environment variable read by [`Alpaca::from_env`] for the API secret key.
pub const ALPACA_SECRET_KEY_ENV: &str = "ALPACA_API_SECRET_KEY";

/// Alpaca market data feed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlpacaFeed {
    /// IEX exchange only. Available on the free plan.
    #[default]
    Iex,
    /// All US exchanges. Requires a paid subscription.
    Sip,
    /// All US exchanges, 15 minutes delayed.
    DelayedSip,
}

impl AlpacaFeed {
    fn as_str(self) -> &'static str {
        match self {
            AlpacaFeed::Iex => "iex",
            AlpacaFeed::Sip => "sip",
            AlpacaFeed::DelayedSip => "delayed_sip",
        }
    }
}

/// US stock prices from the Alpaca Market Data API (`/v2/stocks/snapshots`).
///
/// Needs a free Alpaca account: create API keys in the Alpaca dashboard and pass them to
/// [`Alpaca::new`], or set `ALPACA_API_KEY_ID` / `ALPACA_API_SECRET_KEY` and use
/// [`Alpaca::from_env`].
///
/// Each snapshot carries the latest quote, the latest trade and the daily bar. The returned
/// [`StockPrice::price`] is:
/// 1. the bid/ask midpoint, when both sides are present and the spread is at most
///    [`with_max_spread`](Self::with_max_spread) (2% by default);
/// 2. otherwise the latest trade price;
/// 3. otherwise the daily bar close.
///
/// `bid_price` / `ask_price` are only filled in case 1. On the free IEX feed many tickers
/// have no usable quote outside US market hours, so cases 2 and 3 are common then.
pub struct Alpaca {
    client: reqwest::Client,
    key_id: String,
    secret_key: String,
    feed: AlpacaFeed,
    max_spread: f64,
}

impl Alpaca {
    pub fn new(key_id: impl Into<String>, secret_key: impl Into<String>) -> Self {
        Self {
            client: create_http_client(),
            key_id: key_id.into(),
            secret_key: secret_key.into(),
            feed: AlpacaFeed::default(),
            max_spread: DEFAULT_MAX_SPREAD,
        }
    }

    /// Reads credentials from `ALPACA_API_KEY_ID` and `ALPACA_API_SECRET_KEY`.
    /// Load a `.env` file yourself first (e.g. `dotenvy::dotenv()`) if you keep them there.
    pub fn from_env() -> Result<Self, MarketScannerError> {
        Ok(Self::new(
            credential_from_env(ALPACA_KEY_ID_ENV, "Alpaca")?,
            credential_from_env(ALPACA_SECRET_KEY_ENV, "Alpaca")?,
        ))
    }

    /// Selects the data feed (default: [`AlpacaFeed::Iex`]).
    pub fn with_feed(mut self, feed: AlpacaFeed) -> Self {
        self.feed = feed;
        self
    }

    /// Widest bid/ask spread, as a fraction of mid (default [`DEFAULT_MAX_SPREAD`], 2%),
    /// still used as the price. Wider quotes are treated as stale and the latest trade is
    /// used instead.
    pub fn with_max_spread(mut self, max_spread: f64) -> Self {
        self.max_spread = max_spread;
        self
    }

    /// Like [`fetch_snapshots`](Self::fetch_snapshots), but when Alpaca rejects the batch
    /// because of one invalid ticker, splits it in halves and retries, so only the offending
    /// tickers are dropped.
    fn fetch_snapshots_skipping_invalid<'a>(
        &'a self,
        symbols: &'a [String],
    ) -> futures::future::BoxFuture<'a, Result<AlpacaSnapshotsResponse, MarketScannerError>> {
        Box::pin(async move {
            match self.fetch_snapshots(symbols).await {
                Err(MarketScannerError::InvalidSymbol(_)) if symbols.len() > 1 => {
                    let (left, right) = symbols.split_at(symbols.len() / 2);
                    let (left, right) = futures::future::try_join(
                        self.fetch_snapshots_skipping_invalid(left),
                        self.fetch_snapshots_skipping_invalid(right),
                    )
                    .await?;
                    let mut merged = left;
                    merged.extend(right);
                    Ok(merged)
                }
                Err(MarketScannerError::InvalidSymbol(_)) => Ok(AlpacaSnapshotsResponse::new()),
                other => other,
            }
        })
    }

    async fn fetch_snapshots(
        &self,
        symbols: &[String],
    ) -> Result<AlpacaSnapshotsResponse, MarketScannerError> {
        let url = format!("{ALPACA_DATA_API_BASE}/stocks/snapshots");
        let response = self
            .client
            .get(&url)
            .header("APCA-API-KEY-ID", &self.key_id)
            .header("APCA-API-SECRET-KEY", &self.secret_key)
            .query(&[
                ("symbols", symbols.join(",")),
                ("feed", self.feed.as_str().into()),
            ])
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            if status == reqwest::StatusCode::BAD_REQUEST
                && body.to_lowercase().contains("invalid symbol")
            {
                return Err(MarketScannerError::InvalidSymbol(format!("Alpaca: {body}")));
            }
            return Err(MarketScannerError::ApiError(format!(
                "Alpaca API error: {status} - {body}"
            )));
        }

        Ok(response.json().await?)
    }
}

#[async_trait]
impl StockTrait for Alpaca {
    fn provider(&self) -> StockProvider {
        StockProvider::Alpaca
    }

    fn provider_name(&self) -> &str {
        "Alpaca"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.fetch_snapshots(&["SPY".to_string()]).await.map(|_| ())
    }

    async fn get_price(&self, symbol: &str) -> Result<StockPrice, MarketScannerError> {
        let symbol = normalize_stock_symbol(symbol)?;
        let snapshots = self.fetch_snapshots(std::slice::from_ref(&symbol)).await?;
        snapshots
            .get(&symbol)
            .and_then(|s| to_stock_price(&symbol, s, self.max_spread))
            .ok_or_else(|| {
                MarketScannerError::InvalidSymbol(format!("Alpaca has no price for {symbol}"))
            })
    }

    /// Batched: up to 200 tickers per request, requests sent concurrently.
    async fn get_prices(&self, symbols: &[&str]) -> Result<Vec<StockPrice>, MarketScannerError> {
        let normalized = normalize_stock_symbols(symbols);

        let responses = futures::future::try_join_all(
            normalized
                .chunks(MAX_SYMBOLS_PER_REQUEST)
                .map(|chunk| self.fetch_snapshots_skipping_invalid(chunk)),
        )
        .await?;

        let mut snapshots = AlpacaSnapshotsResponse::new();
        for response in responses {
            snapshots.extend(response);
        }

        Ok(normalized
            .iter()
            .filter_map(|symbol| to_stock_price(symbol, snapshots.get(symbol)?, self.max_spread))
            .collect())
    }
}

/// Daily bars start at midnight New York time; the regular session closes 16 hours later
/// (early-close days excepted). Midnight is already in local time, so this holds across DST.
const SESSION_CLOSE_AFTER_BAR_START_MS: u64 = 16 * 60 * 60 * 1000;

fn session_close(bar_start_ms: u64) -> u64 {
    bar_start_ms + SESSION_CLOSE_AFTER_BAR_START_MS
}

fn positive(value: f64) -> Option<f64> {
    (value.is_finite() && value > 0.0).then_some(value)
}

/// Picks the price as documented on [`Alpaca`]; `None` when the snapshot has no price at all.
fn to_stock_price(symbol: &str, snapshot: &AlpacaSnapshot, max_spread: f64) -> Option<StockPrice> {
    let quote = snapshot
        .latest_quote
        .as_ref()
        .and_then(|q| usable_quote_mid(q.bp, q.ap, max_spread).map(|mid| (q, mid)));
    let trade = snapshot
        .latest_trade
        .as_ref()
        .and_then(|t| positive(t.p).map(|p| (t, p)));
    let bar = snapshot
        .daily_bar
        .as_ref()
        .and_then(|b| positive(b.c).map(|c| (b, c)));

    let (price, exchange_timestamp) = match (quote, trade, bar) {
        (Some((q, mid)), _, _) => (mid, q.t.as_deref().and_then(iso8601_ms)),
        (None, Some((t, p)), _) => (p, t.t.as_deref().and_then(iso8601_ms)),
        (None, None, Some((b, c))) => (c, b.t.as_deref().and_then(iso8601_ms).map(session_close)),
        (None, None, None) => return None,
    };

    Some(StockPrice {
        symbol: symbol.to_string(),
        price,
        bid_price: quote.map(|(q, _)| q.bp),
        ask_price: quote.map(|(q, _)| q.ap),
        bid_qty: quote.map(|(q, _)| q.bs),
        ask_qty: quote.map(|(q, _)| q.ask_size),
        last_price: trade.map(|(_, p)| p),
        timestamp: get_timestamp_millis(),
        exchange_timestamp,
        provider: StockProvider::Alpaca,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of a live `/v2/stocks/snapshots?feed=iex` response, taken while the
    /// market was closed: NVDA has a tight quote, MSFT a stale 10% wide one, AAPL no ask.
    const SNAPSHOTS: &str = r#"{
      "NVDA": {
        "latestQuote": {"ap":159.74,"as":100,"ax":"V","bp":158.0,"bs":200,"bx":"V","c":["R"],"t":"2026-11-17T21:43:49.644498256Z","z":"C"},
        "latestTrade": {"c":["@"],"i":1,"p":158.87,"s":10,"t":"2026-11-17T20:59:59.5Z","x":"V","z":"C"},
        "dailyBar": {"c":158.87,"h":160,"l":157,"n":1,"o":159,"t":"2026-11-17T05:00:00Z","v":1,"vw":158.5}
      },
      "MSFT": {
        "latestQuote": {"ap":530.96,"as":40,"ax":"V","bp":478.04,"bs":40,"bx":"V","c":["R"],"t":"2026-11-17T21:00:00.01067757Z","z":"C"},
        "latestTrade": {"c":["@"],"i":26298,"p":506.07,"s":90,"t":"2026-11-17T20:59:58.02717701Z","x":"V","z":"C"},
        "dailyBar": {"c":506.07,"h":515.515,"l":496.56,"n":26307,"o":514.565,"t":"2026-11-17T05:00:00Z","v":1275244,"vw":504.405499}
      },
      "AAPL": {
        "latestQuote": {"ap":0,"as":0,"ax":" ","bp":335.01,"bs":40,"bx":"V","c":["R"],"t":"2026-11-17T21:00:05.078913366Z","z":"C"},
        "dailyBar": {"c":351.92,"h":355.31,"l":349.77,"n":26024,"o":352.755,"t":"2026-11-17T05:00:00Z","v":1195428,"vw":351.723932}
      },
      "EMPTY": {}
    }"#;

    fn snapshots() -> AlpacaSnapshotsResponse {
        serde_json::from_str(SNAPSHOTS).unwrap()
    }

    #[test]
    fn tight_quote_uses_mid() {
        let p = to_stock_price("NVDA", &snapshots()["NVDA"], DEFAULT_MAX_SPREAD).unwrap();
        assert!((p.price - 158.87).abs() < 1e-9);
        assert_eq!(p.bid_price, Some(158.0));
        assert_eq!(p.ask_price, Some(159.74));
        assert_eq!(p.bid_qty, Some(200.0));
        assert_eq!(p.last_price, Some(158.87));
        assert_eq!(p.exchange_timestamp, Some(1_794_951_829_644));
    }

    #[test]
    fn wide_quote_falls_back_to_last_trade() {
        let p = to_stock_price("MSFT", &snapshots()["MSFT"], DEFAULT_MAX_SPREAD).unwrap();
        assert_eq!(p.price, 506.07);
        assert!(p.bid_price.is_none() && p.ask_price.is_none());
        assert_eq!(p.last_price, Some(506.07));
        assert_eq!(p.exchange_timestamp, Some(1_794_949_198_027));

        // A looser limit accepts the same quote.
        let p = to_stock_price("MSFT", &snapshots()["MSFT"], 0.2).unwrap();
        assert_eq!(p.bid_price, Some(478.04));
        assert!((p.price - 504.5).abs() < 1e-9);
    }

    #[test]
    fn missing_ask_and_trade_falls_back_to_daily_close() {
        let p = to_stock_price("AAPL", &snapshots()["AAPL"], DEFAULT_MAX_SPREAD).unwrap();
        assert_eq!(p.price, 351.92);
        assert!(p.bid_price.is_none() && p.last_price.is_none());
        // Bar starts 2026-11-17T05:00Z (midnight New York); the close is 21:00Z.
        assert_eq!(p.exchange_timestamp, Some(1_794_949_200_000));
    }

    #[test]
    fn empty_snapshot_has_no_price() {
        assert!(to_stock_price("EMPTY", &snapshots()["EMPTY"], DEFAULT_MAX_SPREAD).is_none());
    }
}
