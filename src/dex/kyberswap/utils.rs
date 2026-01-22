use crate::common::MarketScannerError;
use rust_decimal::Decimal;
use std::str::FromStr;

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

/// Helper function to convert wei (raw amount string) to decimal amount
pub fn wei_to_eth(wei_str: &str, decimals: u8) -> Result<f64, MarketScannerError> {
    let wei_decimal = Decimal::from_str(wei_str).map_err(|e| {
        MarketScannerError::ApiError(format!("Invalid wei value '{}': {}", wei_str, e))
    })?;
    let divisor_str = format!("1{}", "0".repeat(decimals as usize));
    let divisor = Decimal::from_str(&divisor_str).map_err(|e| {
        MarketScannerError::ApiError(format!("Failed to create divisor 10^{}: {}", decimals, e))
    })?;
    let result = wei_decimal
        .checked_div(divisor)
        .ok_or_else(|| MarketScannerError::ApiError("Division by zero or overflow".to_string()))?;

    result.to_string().parse::<f64>().map_err(|e| {
        MarketScannerError::ApiError(format!("Failed to convert Decimal to f64: {}", e))
    })
}

/// Safe division for price calculations
pub fn safe_divide(numerator: f64, divisor: f64) -> Result<f64, MarketScannerError> {
    if divisor == 0.0 {
        return Err(MarketScannerError::ApiError(
            "Division by zero: divisor cannot be zero".to_string(),
        ));
    }

    // Convert to Decimal for precision-safe division
    let num_decimal = Decimal::from_f64_retain(numerator)
        .ok_or_else(|| MarketScannerError::ApiError(format!("Invalid numerator: {}", numerator)))?;

    let div_decimal = Decimal::from_f64_retain(divisor)
        .ok_or_else(|| MarketScannerError::ApiError(format!("Invalid divisor: {}", divisor)))?;

    // Perform division with Decimal
    let result = num_decimal.checked_div(div_decimal).ok_or_else(|| {
        MarketScannerError::ApiError("Division resulted in invalid value".to_string())
    })?;

    // Convert back to f64
    result.to_string().parse::<f64>().map_err(|e| {
        MarketScannerError::ApiError(format!("Failed to convert Decimal to f64: {}", e))
    })
}
