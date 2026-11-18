mod types;

use async_trait::async_trait;

use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::fiat::exchangerate_api::types::ErApiLatestResponse;
use crate::fiat::{FiatProvider, FiatRate, FiatTrait, parse_currency_code, parse_fiat_pair};

const ER_API_BASE: &str = "https://open.er-api.com/v6";

/// Fiat exchange rates from the [ExchangeRate-API](https://www.exchangerate-api.com/docs/free)
/// open access endpoint: 160+ currencies, no API key.
///
/// Rates are refreshed once per day. Terms of the open access endpoint:
/// - Attribution is required on pages that use the rates:
///   `<a href="https://www.exchangerate-api.com">Rates By Exchange Rate API</a>`.
/// - Requesting once per hour never gets rate limited; heavier callers get HTTP 429 for
///   20 minutes. Every call downloads the base currency's table, so **cache the results on
///   your side** (the data only changes once per day) rather than calling in a loop.
///   [`FiatRate::timestamp`] is when the table was downloaded.
/// - Personal and commercial use is allowed; re-distributing the data is not.
pub struct ExchangeRateApi {
    client: reqwest::Client,
}

impl Default for ExchangeRateApi {
    fn default() -> Self {
        Self::new()
    }
}

impl ExchangeRateApi {
    pub fn new() -> Self {
        Self {
            client: create_http_client(),
        }
    }

    /// Every rate available against `base` (160+ currencies), sorted by quote code.
    pub async fn get_all_rates(&self, base: &str) -> Result<Vec<FiatRate>, MarketScannerError> {
        let base = parse_currency_code(base)?;
        let response = self.fetch_latest(&base).await?;
        let mut quotes: Vec<&String> = response.rates.keys().filter(|q| **q != base).collect();
        quotes.sort();
        Ok(quotes
            .into_iter()
            .filter_map(|quote| to_rate(&base, &response, quote))
            .collect())
    }

    async fn fetch_latest(&self, base: &str) -> Result<ErApiLatestResponse, MarketScannerError> {
        let url = format!("{ER_API_BASE}/latest/{base}");
        let response = self.client.get(&url).send().await?;
        let status = response.status();
        let text = response.text().await?;
        // Read the body first: errors carry `"result": "error"` whatever the HTTP status.
        let parsed: Option<ErApiLatestResponse> = serde_json::from_str(&text).ok();

        if let Some(body) = parsed.as_ref().filter(|b| b.result != "success") {
            let kind = body.error_type.as_deref().unwrap_or("unknown");
            return Err(if kind == "unsupported-code" {
                MarketScannerError::InvalidSymbol(format!(
                    "ExchangeRate-API does not support currency {base}"
                ))
            } else {
                MarketScannerError::ApiError(format!("ExchangeRate-API error: {status} {kind}"))
            });
        }
        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MarketScannerError::ApiError(
                "ExchangeRate-API rate limited this IP (HTTP 429) for about 20 minutes".to_string(),
            ));
        }
        if !status.is_success() {
            return Err(MarketScannerError::ApiError(format!(
                "ExchangeRate-API error: {status} - {}",
                text.chars().take(200).collect::<String>()
            )));
        }

        let mut body = parsed.ok_or_else(|| {
            MarketScannerError::ApiError("ExchangeRate-API: could not parse response".to_string())
        })?;
        body.fetched_at = get_timestamp_millis();
        Ok(body)
    }
}

#[async_trait]
impl FiatTrait for ExchangeRateApi {
    fn provider(&self) -> FiatProvider {
        FiatProvider::ExchangeRateApi
    }

    fn provider_name(&self) -> &str {
        "ExchangeRate-API"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_rate("USD/EUR").await.map(|_| ())
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let (base, quote) = parse_fiat_pair(pair)?;
        let response = self.fetch_latest(&base).await?;
        to_rate(&base, &response, &quote).ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!(
                "ExchangeRate-API has no {base}/{quote} rate"
            ))
        })
    }

    /// One request per distinct base currency; results keep input order.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        let parsed = pairs
            .iter()
            .map(|pair| parse_fiat_pair(pair))
            .collect::<Result<Vec<_>, _>>()?;

        let mut bases: Vec<&String> = Vec::new();
        for (base, _) in &parsed {
            if !bases.contains(&base) {
                bases.push(base);
            }
        }
        let responses =
            futures::future::try_join_all(bases.iter().map(|base| self.fetch_latest(base))).await?;

        parsed
            .iter()
            .map(|(base, quote)| {
                let index = bases.iter().position(|b| *b == base).expect("base listed");
                to_rate(base, &responses[index], quote).ok_or_else(|| {
                    MarketScannerError::InvalidSymbol(format!(
                        "ExchangeRate-API has no {base}/{quote} rate"
                    ))
                })
            })
            .collect()
    }
}

fn to_rate(base: &str, response: &ErApiLatestResponse, quote: &str) -> Option<FiatRate> {
    let rate = *response.rates.get(quote)?;
    if rate <= 0.0 {
        return None;
    }
    let base = response.base_code.as_deref().unwrap_or(base);
    let updated = response
        .time_last_update_unix
        .and_then(|s| chrono::DateTime::from_timestamp(s, 0));
    Some(FiatRate {
        pair: format!("{base}/{quote}"),
        base: base.to_string(),
        quote: quote.to_string(),
        rate,
        bid: None,
        ask: None,
        date: updated
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default(),
        timestamp: response.fetched_at,
        exchange_timestamp: updated.and_then(|d| u64::try_from(d.timestamp_millis()).ok()),
        provider: FiatProvider::ExchangeRateApi,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rate_from_response() {
        let json = r#"{"result":"success","base_code":"USD","time_last_update_unix":1794960151,
            "rates":{"USD":1,"EUR":0.91,"TRY":50.4}}"#;
        let response: ErApiLatestResponse = serde_json::from_str(json).unwrap();
        let rate = to_rate("USD", &response, "TRY").unwrap();
        assert_eq!(rate.pair, "USD/TRY");
        assert_eq!(rate.rate, 50.4);
        assert_eq!(rate.date, "2026-11-18");
        assert_eq!(rate.exchange_timestamp, Some(1_794_960_151_000));
        assert_eq!(rate.provider, FiatProvider::ExchangeRateApi);
        assert!(to_rate("USD", &response, "JPY").is_none());
    }

    #[test]
    fn parses_error_response() {
        let json = r#"{"result":"error","error-type":"unsupported-code"}"#;
        let response: ErApiLatestResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.result, "error");
        assert_eq!(response.error_type.as_deref(), Some("unsupported-code"));
    }
}
