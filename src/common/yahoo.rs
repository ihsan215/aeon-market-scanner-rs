//! Shared HTTP client for the **unofficial** Yahoo Finance chart endpoint, used by
//! `fiat::yahoo` and `stocks::yahoo` (feature `unofficial-yahoo`).

use serde::Deserialize;

use crate::common::MarketScannerError;

const YAHOO_CHART_BASE: &str = "https://query1.finance.yahoo.com/v8/finance/chart";

/// Requests in flight at once for multi-symbol calls; kept low to avoid Yahoo's rate limits.
pub(crate) const YAHOO_MAX_CONCURRENT_REQUESTS: usize = 4;

#[derive(Debug, Deserialize)]
pub(crate) struct YahooChartResponse {
    pub chart: YahooChart,
}

#[derive(Debug, Deserialize)]
pub(crate) struct YahooChart {
    #[serde(default)]
    pub result: Option<Vec<YahooChartResult>>,
    #[serde(default)]
    pub error: Option<YahooError>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct YahooChartResult {
    pub meta: YahooMeta,
}

/// The `meta` block of a chart response (only the fields we use).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct YahooMeta {
    #[serde(default)]
    pub regular_market_price: Option<f64>,
    /// Unix seconds of the last price.
    #[serde(default)]
    pub regular_market_time: Option<i64>,
    /// `CURRENCY`, `EQUITY`, `ETF`, `INDEX`, ...
    #[serde(default)]
    pub instrument_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct YahooError {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

pub(crate) struct YahooClient {
    client: reqwest::Client,
}

impl YahooClient {
    pub fn new() -> Self {
        // Yahoo answers requests without a User-Agent with 429, so identify the crate.
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .user_agent(concat!(
                "aeon-market-scanner-rs/",
                env!("CARGO_PKG_VERSION"),
                " (+https://github.com/ihsan215/aeon-market-scanner-rs)"
            ))
            .build()
            .expect("Failed to create HTTP client");
        Self { client }
    }

    /// Chart `meta` for a Yahoo symbol (`EURUSD=X`, `AAPL`, `BRK-B`). `label` names the
    /// requested instrument in error messages.
    pub async fn fetch_meta(
        &self,
        yahoo_symbol: &str,
        label: &str,
    ) -> Result<YahooMeta, MarketScannerError> {
        let url = format!("{YAHOO_CHART_BASE}/{yahoo_symbol}");
        let response = self
            .client
            .get(&url)
            .query(&[("interval", "1d"), ("range", "1d")])
            .send()
            .await?;
        let status = response.status();

        if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(MarketScannerError::ApiError(
                "Yahoo rate limited the request (HTTP 429); retry later".to_string(),
            ));
        }
        let body = response.text().await?;
        let parsed: Option<YahooChartResponse> = serde_json::from_str(&body).ok();

        let chart_error = parsed.as_ref().and_then(|p| p.chart.error.as_ref());
        let not_found = status == reqwest::StatusCode::NOT_FOUND
            || chart_error.is_some_and(|e| e.code.as_deref() == Some("Not Found"));
        if not_found {
            return Err(MarketScannerError::InvalidSymbol(format!(
                "Yahoo has no data for {label}"
            )));
        }
        if let Some(error) = chart_error {
            let message = format!(
                "Yahoo: {} {}",
                error.code.as_deref().unwrap_or(""),
                error.description.as_deref().unwrap_or("")
            );
            // A 400 with a chart error means Yahoo rejected the symbol itself.
            return Err(if status == reqwest::StatusCode::BAD_REQUEST {
                MarketScannerError::InvalidSymbol(format!("{label}: {message}"))
            } else {
                MarketScannerError::ApiError(message)
            });
        }
        if !status.is_success() {
            return Err(MarketScannerError::ApiError(format!(
                "Yahoo error: {status} - {}",
                body.chars().take(200).collect::<String>()
            )));
        }

        parsed
            .and_then(|p| p.chart.result)
            .and_then(|mut results| results.pop())
            .map(|r| r.meta)
            .ok_or_else(|| MarketScannerError::ApiError(format!("Yahoo: no data for {label}")))
    }
}

/// Unix seconds -> (`YYYY-MM-DD`, ms since epoch).
pub(crate) fn yahoo_time(seconds: Option<i64>) -> (String, Option<u64>) {
    match seconds.and_then(|s| chrono::DateTime::from_timestamp(s, 0)) {
        Some(t) => (
            t.format("%Y-%m-%d").to_string(),
            u64::try_from(t.timestamp_millis()).ok(),
        ),
        None => (String::new(), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_chart_and_error_responses() {
        let ok = r#"{"chart":{"result":[{"meta":{"currency":"USD","symbol":"AAPL",
            "instrumentType":"EQUITY","regularMarketTime":1794949201,
            "regularMarketPrice":351.91}}],"error":null}}"#;
        let response: YahooChartResponse = serde_json::from_str(ok).unwrap();
        let meta = &response.chart.result.unwrap()[0].meta;
        assert_eq!(meta.regular_market_price, Some(351.91));
        assert_eq!(meta.instrument_type.as_deref(), Some("EQUITY"));

        let err = r#"{"chart":{"result":null,"error":{"code":"Not Found","description":"No data found, symbol may be delisted"}}}"#;
        let response: YahooChartResponse = serde_json::from_str(err).unwrap();
        assert!(response.chart.result.is_none());
        assert_eq!(
            response.chart.error.unwrap().code.as_deref(),
            Some("Not Found")
        );
    }

    #[test]
    fn converts_time() {
        assert_eq!(
            yahoo_time(Some(1_794_989_536)),
            ("2026-11-18".to_string(), Some(1_794_989_536_000))
        );
        assert_eq!(yahoo_time(None), (String::new(), None));
    }
}
