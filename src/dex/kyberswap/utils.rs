use crate::common::MarketScannerError;

/// Create HTTP client with browser-like headers to bypass Cloudflare protection
pub fn create_http_client_with_browser_headers() -> Result<reqwest::Client, MarketScannerError> {
    let client = reqwest::Client::builder()
        .default_headers({
            let mut headers = reqwest::header::HeaderMap::new();
            headers.insert(
                "X-Client-Id",
                reqwest::header::HeaderValue::from_static("wc-arbitrage-bot"),
            );
            headers.insert(
                "User-Agent",
                reqwest::header::HeaderValue::from_static(
                    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
                ),
            );
            headers.insert(
                "Accept",
                reqwest::header::HeaderValue::from_static("application/json"),
            );
            headers.insert(
                "Accept-Language",
                reqwest::header::HeaderValue::from_static("en-US,en;q=0.9"),
            );
            headers
        })
        .build()
        .map_err(|e| MarketScannerError::HttpError(e))?;

    Ok(client)
}

/// Helper function to calculate token amount for a USD value
/// Returns string to avoid overflow issues with large decimals
pub fn calculate_amount_for_value(usd_value: f64, decimals: u8) -> String {
    // Format: multiply by 10^decimals as a string
    let base = format!("{:.0}", usd_value).replace(".", "");
    let zeros = "0".repeat(decimals as usize);
    format!("{}{}", base, zeros)
}
