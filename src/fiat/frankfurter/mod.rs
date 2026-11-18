mod types;

use std::collections::HashMap;

use async_trait::async_trait;
use serde::Deserialize;

use crate::common::{MarketScannerError, create_http_client, get_timestamp_millis};
use crate::fiat::frankfurter::types::FrankfurterLatestResponse;
use crate::fiat::{FiatProvider, FiatRate, FiatTrait, parse_currency_code, parse_fiat_pair};

const FRANKFURTER_API_BASE: &str = "https://api.frankfurter.dev/v1";

/// Fiat exchange rates from [Frankfurter](https://frankfurter.dev), which serves the
/// European Central Bank reference rates (about 30 currencies, including USD, EUR, GBP, JPY,
/// CHF, CNY, TRY). No API key needed.
///
/// Rates are updated once per working day around 16:00 CET; use them as reference rates,
/// not as live prices.
pub struct Frankfurter {
    client: reqwest::Client,
}

impl Default for Frankfurter {
    fn default() -> Self {
        Self::new()
    }
}

impl Frankfurter {
    pub fn new() -> Self {
        Self {
            client: create_http_client(),
        }
    }

    /// Every rate available against `base` (e.g. `USD` -> `USD/EUR`, `USD/TRY`, ...).
    pub async fn get_all_rates(&self, base: &str) -> Result<Vec<FiatRate>, MarketScannerError> {
        let base = parse_currency_code(base)?;
        let response = self.fetch_latest(&base, None).await?;
        let mut quotes: Vec<&String> = response.rates.keys().collect();
        quotes.sort();
        Ok(quotes
            .into_iter()
            .filter_map(|quote| to_rate(&response, quote))
            .collect())
    }

    /// Supported currency codes mapped to their names (`"USD" -> "United States Dollar"`).
    pub async fn supported_currencies(
        &self,
    ) -> Result<HashMap<String, String>, MarketScannerError> {
        self.get_json(&format!("{FRANKFURTER_API_BASE}/currencies"), &[])
            .await
    }

    async fn fetch_latest(
        &self,
        base: &str,
        quotes: Option<&[String]>,
    ) -> Result<FrankfurterLatestResponse, MarketScannerError> {
        let mut query = vec![("base", base.to_string())];
        if let Some(quotes) = quotes {
            query.push(("symbols", quotes.join(",")));
        }
        self.get_json(&format!("{FRANKFURTER_API_BASE}/latest"), &query)
            .await
    }

    async fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        query: &[(&str, String)],
    ) -> Result<T, MarketScannerError> {
        let response = self.client.get(url).query(query).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            // Unknown currency codes come back as 404 / 422.
            if status == reqwest::StatusCode::NOT_FOUND
                || status == reqwest::StatusCode::UNPROCESSABLE_ENTITY
            {
                return Err(MarketScannerError::InvalidSymbol(format!(
                    "Frankfurter: {status} - {body}"
                )));
            }
            return Err(MarketScannerError::ApiError(format!(
                "Frankfurter API error: {status} - {body}"
            )));
        }
        Ok(response.json().await?)
    }
}

#[async_trait]
impl FiatTrait for Frankfurter {
    fn provider(&self) -> FiatProvider {
        FiatProvider::Frankfurter
    }

    fn provider_name(&self) -> &str {
        "Frankfurter"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_rate("EUR/USD").await.map(|_| ())
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let (base, quote) = parse_fiat_pair(pair)?;
        let response = self
            .fetch_latest(&base, Some(std::slice::from_ref(&quote)))
            .await?;
        to_rate(&response, &quote).ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("Frankfurter has no {base}/{quote} rate"))
        })
    }

    /// Pairs sharing a base currency go in one request; results keep input order.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        let parsed = pairs
            .iter()
            .map(|pair| parse_fiat_pair(pair))
            .collect::<Result<Vec<_>, _>>()?;

        let mut quotes_by_base: Vec<(String, Vec<String>)> = Vec::new();
        for (base, quote) in &parsed {
            match quotes_by_base.iter_mut().find(|(b, _)| b == base) {
                Some((_, quotes)) if !quotes.contains(quote) => quotes.push(quote.clone()),
                Some(_) => {}
                None => quotes_by_base.push((base.clone(), vec![quote.clone()])),
            }
        }

        let responses = futures::future::try_join_all(
            quotes_by_base
                .iter()
                .map(|(base, quotes)| self.fetch_latest(base, Some(quotes))),
        )
        .await?;

        parsed
            .iter()
            .map(|(base, quote)| {
                responses
                    .iter()
                    .find(|r| &r.base == base)
                    .and_then(|r| to_rate(r, quote))
                    .ok_or_else(|| {
                        MarketScannerError::InvalidSymbol(format!(
                            "Frankfurter has no {base}/{quote} rate"
                        ))
                    })
            })
            .collect()
    }
}

fn to_rate(response: &FrankfurterLatestResponse, quote: &str) -> Option<FiatRate> {
    let rate = *response.rates.get(quote)?;
    Some(FiatRate {
        pair: format!("{}/{}", response.base, quote),
        base: response.base.clone(),
        quote: quote.to_string(),
        rate,
        bid: None,
        ask: None,
        date: response.date.clone(),
        timestamp: get_timestamp_millis(),
        exchange_timestamp: None,
        provider: FiatProvider::Frankfurter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_rate_from_response() {
        let json =
            r#"{"amount":1.0,"base":"EUR","date":"2026-11-17","rates":{"USD":1.0989,"TRY":55.4}}"#;
        let response: FrankfurterLatestResponse = serde_json::from_str(json).unwrap();
        let rate = to_rate(&response, "USD").unwrap();
        assert_eq!(rate.pair, "EUR/USD");
        assert_eq!(rate.rate, 1.0989);
        assert_eq!(rate.date, "2026-11-17");
        assert_eq!(rate.provider, FiatProvider::Frankfurter);
        assert!(to_rate(&response, "JPY").is_none());
    }
}
