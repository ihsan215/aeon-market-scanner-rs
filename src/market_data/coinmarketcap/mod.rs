mod types;

use std::collections::HashMap;

use async_trait::async_trait;

use crate::common::utils::iso8601_ms;
use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::market_data::coinmarketcap::types::{CmcAsset, CmcResponse};
use crate::market_data::{
    MarketDataPrice, MarketDataProvider, MarketDataTrait, normalize_asset_symbol,
};

const CMC_KEYED_BASE: &str = "https://pro-api.coinmarketcap.com/v3";
const CMC_KEYLESS_BASE: &str = "https://pro-api.coinmarketcap.com/public-api/v3";

/// Symbols sent per request. A symbol lookup returns every asset sharing the symbol, so the
/// response is several times larger than the request.
const MAX_SYMBOLS_PER_REQUEST: usize = 100;

/// Environment variable read by [`CoinMarketCap::from_env`].
pub const CMC_API_KEY_ENV: &str = "CMC_API_KEY";

/// Aggregated crypto prices from the [CoinMarketCap API](https://coinmarketcap.com/api/)
/// (`/v3/cryptocurrency/quotes/latest`), with market cap, 24h volume and 24h change.
///
/// - **No key needed**: [`CoinMarketCap::new`] uses the keyless public API. Its anonymous
///   quota is small and IP-based; once used up it answers HTTP 429 with code 1022 until it
///   resets. For anything beyond occasional calls, use a free key.
/// - With a free API key ([`with_api_key`](Self::with_api_key) or `CMC_API_KEY`), the free
///   Basic plan gives 15,000 credits/month and 50 requests/minute. Each call costs 1 credit
///   per 250 assets returned.
/// - Prices are refreshed every 15–60 seconds. There is no WebSocket on the free plans.
/// - Symbols are not unique on CoinMarketCap (many tokens call themselves `BTC`). For each
///   symbol this provider returns the active asset with the best market-cap rank, which is
///   the well-known one. Use [`get_prices_by_ids`](Self::get_prices_by_ids) for exact assets.
/// - Licence: per CoinMarketCap's pricing page, the free plan allows commercial use inside
///   one product, but not redistributing the data as a standalone service or through your own
///   API. Check the current terms before displaying the data.
pub struct CoinMarketCap {
    client: reqwest::Client,
    api_key: Option<String>,
    convert: String,
}

impl Default for CoinMarketCap {
    fn default() -> Self {
        Self::new()
    }
}

impl CoinMarketCap {
    /// Keyless client (shared, IP-based rate limit).
    pub fn new() -> Self {
        Self {
            client: create_http_client(),
            api_key: None,
            convert: "USD".to_string(),
        }
    }

    /// Client using your API key (higher, per-key limits).
    pub fn with_api_key(api_key: impl Into<String>) -> Self {
        Self {
            api_key: Some(api_key.into()),
            ..Self::new()
        }
    }

    /// Uses `CMC_API_KEY` when it is set, otherwise the keyless API.
    pub fn from_env() -> Self {
        match std::env::var(CMC_API_KEY_ENV) {
            Ok(key) if !key.trim().is_empty() => Self::with_api_key(key.trim()),
            _ => Self::new(),
        }
    }

    /// Currency to quote prices in (default `USD`), e.g. `EUR` or `BTC`. The free plan allows
    /// one per call.
    pub fn with_convert(mut self, currency: impl Into<String>) -> Self {
        self.convert = currency.into().trim().to_uppercase();
        self
    }

    /// Whether requests use an API key.
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Prices for exact CoinMarketCap ids (`1` = Bitcoin, `1027` = Ethereum), in input order.
    /// Unknown ids are left out.
    pub async fn get_prices_by_ids(
        &self,
        ids: &[u64],
    ) -> Result<Vec<MarketDataPrice>, MarketScannerError> {
        let mut unique: Vec<u64> = Vec::new();
        for id in ids {
            if !unique.contains(id) {
                unique.push(*id);
            }
        }
        let mut by_id: HashMap<u64, MarketDataPrice> = HashMap::new();
        for chunk in unique.chunks(MAX_SYMBOLS_PER_REQUEST) {
            let ids = chunk
                .iter()
                .map(u64::to_string)
                .collect::<Vec<_>>()
                .join(",");
            for asset in self.fetch_quotes("id", &ids).await? {
                if let Some(price) = self.to_price(&asset) {
                    by_id.insert(asset.id, price);
                }
            }
        }
        Ok(unique.iter().filter_map(|id| by_id.remove(id)).collect())
    }

    async fn fetch_quotes(
        &self,
        by: &str,
        values: &str,
    ) -> Result<Vec<CmcAsset>, MarketScannerError> {
        let base = if self.api_key.is_some() {
            CMC_KEYED_BASE
        } else {
            CMC_KEYLESS_BASE
        };
        let mut request = self
            .client
            .get(format!("{base}/cryptocurrency/quotes/latest"))
            .header("Accept", "application/json")
            .query(&[
                (by, values),
                ("convert", self.convert.as_str()),
                ("skip_invalid", "true"),
            ]);
        if let Some(key) = &self.api_key {
            request = request.header("X-CMC_PRO_API_KEY", key);
        }
        let response = request.send().await?;
        let status = response.status();
        let text = response.text().await?;
        let parsed: Option<CmcResponse<Vec<CmcAsset>>> = serde_json::from_str(&text).ok();

        let (code, message) = parsed
            .as_ref()
            .map(|p| {
                (
                    p.status.code(),
                    p.status.error_message.clone().unwrap_or_default(),
                )
            })
            .unwrap_or((0, String::new()));
        if code != 0 || !status.is_success() {
            let hint = match code {
                1001 | 1002 => " (check the API key)",
                1008 => " (per-minute limit; wait 60 seconds)",
                1009 | 1010 => " (daily or monthly credit limit reached)",
                1011 => " (IP rate limit; reduce concurrent requests)",
                1022 => " (keyless limit reached; set CMC_API_KEY to a free API key)",
                _ if status == reqwest::StatusCode::TOO_MANY_REQUESTS => {
                    " (rate limited; back off and retry)"
                }
                _ => "",
            };
            let detail = if message.is_empty() {
                text.chars().take(200).collect()
            } else {
                message
            };
            return Err(MarketScannerError::ApiError(format!(
                "CoinMarketCap API error: {status} code {code}{hint} - {detail}"
            )));
        }

        Ok(parsed.and_then(|p| p.data).unwrap_or_default())
    }

    fn to_price(&self, asset: &CmcAsset) -> Option<MarketDataPrice> {
        let quote = asset
            .quote
            .iter()
            .find(|q| q.symbol.eq_ignore_ascii_case(&self.convert))
            .or_else(|| asset.quote.first())?;
        let price = quote.price.filter(|p| p.is_finite() && *p > 0.0)?;
        Some(MarketDataPrice {
            symbol: asset.symbol.to_uppercase(),
            name: asset.name.clone(),
            provider_id: asset.id.to_string(),
            quote: quote.symbol.to_uppercase(),
            price,
            market_cap: quote.market_cap.filter(|v| *v > 0.0),
            volume_24h: quote.volume_24h.filter(|v| *v >= 0.0),
            percent_change_24h: quote.percent_change_24h,
            rank: asset.cmc_rank,
            timestamp: get_timestamp_millis(),
            exchange_timestamp: quote.last_updated.as_deref().and_then(iso8601_ms),
            provider: MarketDataProvider::CoinMarketCap,
        })
    }
}

/// For each symbol, the active asset with a price and the best (lowest) rank; unranked assets
/// lose to ranked ones, then larger market cap wins.
fn pick_best_per_symbol(prices: Vec<(bool, MarketDataPrice)>) -> HashMap<String, MarketDataPrice> {
    let mut best: HashMap<String, MarketDataPrice> = HashMap::new();
    for (active, price) in prices {
        if !active {
            continue;
        }
        let better = match best.get(&price.symbol) {
            None => true,
            Some(current) => {
                let key = |p: &MarketDataPrice| {
                    (
                        p.rank.unwrap_or(u32::MAX),
                        std::cmp::Reverse(p.market_cap.unwrap_or(0.0).to_bits()),
                    )
                };
                key(&price) < key(current)
            }
        };
        if better {
            best.insert(price.symbol.clone(), price);
        }
    }
    best
}

#[async_trait]
impl MarketDataTrait for CoinMarketCap {
    fn provider(&self) -> MarketDataProvider {
        MarketDataProvider::CoinMarketCap
    }

    fn provider_name(&self) -> &str {
        "CoinMarketCap"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_price("BTC").await.map(|_| ())
    }

    async fn get_price(&self, symbol: &str) -> Result<MarketDataPrice, MarketScannerError> {
        let symbol = normalize_asset_symbol(symbol)?;
        self.get_prices(&[&symbol]).await?.pop().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("CoinMarketCap has no price for {symbol}"))
        })
    }

    /// Up to 100 symbols per request. Malformed and unknown symbols are left out.
    async fn get_prices(
        &self,
        symbols: &[&str],
    ) -> Result<Vec<MarketDataPrice>, MarketScannerError> {
        let mut wanted: Vec<String> = Vec::new();
        for symbol in symbols {
            if let Ok(s) = normalize_asset_symbol(symbol) {
                if !wanted.contains(&s) {
                    wanted.push(s);
                }
            }
        }

        let mut candidates = Vec::new();
        for chunk in wanted.chunks(MAX_SYMBOLS_PER_REQUEST) {
            for asset in self.fetch_quotes("symbol", &chunk.join(",")).await? {
                if let Some(price) = self.to_price(&asset) {
                    candidates.push((asset.is_active.unwrap_or(1) == 1, price));
                }
            }
        }
        let mut best = pick_best_per_symbol(candidates);
        Ok(wanted.iter().filter_map(|s| best.remove(s)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Trimmed copy of a live keyless response for `symbol=BTC,ETH`: the real assets plus
    /// look-alikes (inactive, unranked, bridged).
    const QUOTES: &str = r#"{"data":[
      {"id":1,"name":"Bitcoin","symbol":"BTC","cmc_rank":1,"is_active":1,
       "quote":[{"symbol":"USD","price":64136.11917710928,"market_cap":1290007220502.7983,"volume_24h":31000000000.5,"percent_change_24h":-1.25,"last_updated":"2026-11-18T11:44:01.000Z"}]},
      {"id":1027,"name":"Ethereum","symbol":"ETH","cmc_rank":2,"is_active":1,
       "quote":[{"symbol":"USD","price":1797.632518421438,"market_cap":219725304144.34543,"last_updated":"2026-11-18T11:44:01.000Z"}]},
      {"id":30938,"name":"Satoshi Pumpomoto","symbol":"BTC","cmc_rank":null,"is_active":0,
       "quote":[{"symbol":"USD","price":null,"market_cap":null,"last_updated":"2026-11-18T11:44:01.000Z"}]},
      {"id":39556,"name":"Bridged BTC","symbol":"BTC","cmc_rank":7950,"is_active":1,
       "quote":[{"symbol":"USD","price":64894.75139212534,"market_cap":0,"last_updated":"2026-11-18T11:44:01.000Z"}]},
      {"id":38552,"name":"Bitcoin Base","symbol":"BTC","cmc_rank":3164,"is_active":1,
       "quote":[{"symbol":"USD","price":0.002902545486548901,"market_cap":60695.659835048114,"last_updated":"2026-11-18T11:44:01.000Z"}]}
    ],"status":{"timestamp":"2026-11-18T11:45:21.962Z","error_code":"0","error_message":"","elapsed":7,"credit_count":1}}"#;

    fn parse(json: &str) -> CmcResponse<Vec<CmcAsset>> {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn picks_the_ranked_asset_for_each_symbol() {
        let cmc = CoinMarketCap::new();
        let assets = parse(QUOTES).data.unwrap();
        let candidates = assets
            .iter()
            .filter_map(|a| Some((a.is_active.unwrap_or(1) == 1, cmc.to_price(a)?)))
            .collect();
        let best = pick_best_per_symbol(candidates);

        let btc = &best["BTC"];
        assert_eq!(btc.provider_id, "1");
        assert_eq!(btc.name, "Bitcoin");
        assert_eq!(btc.quote, "USD");
        assert_eq!(btc.price, 64136.11917710928);
        assert_eq!(btc.rank, Some(1));
        assert_eq!(btc.percent_change_24h, Some(-1.25));
        assert_eq!(btc.exchange_timestamp, Some(1_795_002_241_000));
        assert_eq!(best["ETH"].provider_id, "1027");
        assert!(best["ETH"].volume_24h.is_none());
    }

    #[test]
    fn parses_status_codes_as_number_or_string() {
        let err: CmcResponse<Vec<CmcAsset>> = serde_json::from_str(
            r#"{"status":{"timestamp":"2026-11-18T11:45:04.877Z","error_code":"1002","error_message":"API key missing.","elapsed":0,"credit_count":0}}"#,
        )
        .unwrap();
        assert_eq!(err.status.code(), 1002);
        assert!(err.data.is_none());

        let ok: CmcResponse<Vec<CmcAsset>> =
            serde_json::from_str(r#"{"data":[],"status":{"error_code":0}}"#).unwrap();
        assert_eq!(ok.status.code(), 0);
    }

    #[test]
    fn from_env_without_key_is_keyless() {
        assert!(!CoinMarketCap::new().has_api_key());
        assert!(CoinMarketCap::with_api_key("x").has_api_key());
        assert_eq!(CoinMarketCap::new().with_convert(" eur ").convert, "EUR");
    }
}
