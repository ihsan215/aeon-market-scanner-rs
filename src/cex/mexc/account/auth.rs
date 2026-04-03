use crate::common::{ExchangeTrait, MarketScannerError};
use hmac::{Hmac, Mac};
use reqwest::Method;
use sha2::Sha256;
use std::collections::BTreeMap;

type HmacSha256 = Hmac<Sha256>;

pub(super) const DEFAULT_RECV_WINDOW: u64 = 5_000;

pub(super) async fn signed_request<T: for<'de> serde::Deserialize<'de>>(
    mexc: &super::super::Mexc,
    method: Method,
    endpoint: &str,
    mut params: BTreeMap<String, String>,
) -> Result<T, MarketScannerError> {
    let (api_key, api_secret) = mexc.credentials()?;

    params.insert("recvWindow".to_string(), DEFAULT_RECV_WINDOW.to_string());
    params.insert(
        "timestamp".to_string(),
        crate::common::get_timestamp_millis().to_string(),
    );

    let query = build_query_string(&params);
    let signature = sign_query(&query, api_secret)?;
    let signed_query = format!("{query}&signature={signature}");
    let url = format!("{}/{}?{}", mexc.api_base(), endpoint, signed_query);

    let response = mexc
        .client
        .request(method, &url)
        .header("X-MEXC-APIKEY", api_key)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(MarketScannerError::ApiError(format!(
            "Mexc API error: {} - {}",
            status, body
        )));
    }

    Ok(response.json::<T>().await?)
}

pub(super) async fn create_listen_key(mexc: &super::super::Mexc) -> Result<String, MarketScannerError> {
    let response = signed_raw_request(
        mexc,
        Method::POST,
        "https://api.mexc.com/api/v3/userDataStream",
        BTreeMap::new(),
    )
    .await?;

    Ok(response
        .json::<super::types::MexcListenKeyResponse>()
        .await?
        .listen_key)
}

pub(super) async fn refresh_listen_key(
    mexc: &super::super::Mexc,
    listen_key: &str,
) -> Result<(), MarketScannerError> {
    let mut params = BTreeMap::new();
    params.insert("listenKey".to_string(), listen_key.to_string());

    signed_raw_request(
        mexc,
        Method::PUT,
        "https://api.mexc.com/api/v3/userDataStream",
        params,
    )
    .await?;

    Ok(())
}

async fn signed_raw_request(
    mexc: &super::super::Mexc,
    method: Method,
    url_base: &str,
    mut params: BTreeMap<String, String>,
) -> Result<reqwest::Response, MarketScannerError> {
    let (api_key, api_secret) = mexc.credentials()?;

    params.insert("recvWindow".to_string(), DEFAULT_RECV_WINDOW.to_string());
    params.insert(
        "timestamp".to_string(),
        crate::common::get_timestamp_millis().to_string(),
    );

    let query = build_query_string(&params);
    let signature = sign_query(&query, api_secret)?;
    let signed_query = format!("{query}&signature={signature}");
    let url = format!("{url_base}?{signed_query}");

    let response = mexc
        .client
        .request(method, &url)
        .header("X-MEXC-APIKEY", api_key)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(MarketScannerError::ApiError(format!(
            "Mexc API error: {} - {}",
            status, body
        )));
    }

    Ok(response)
}

fn build_query_string(params: &BTreeMap<String, String>) -> String {
    params
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join("&")
}

fn sign_query(query: &str, secret: &str) -> Result<String, MarketScannerError> {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|err| MarketScannerError::ApiError(format!("invalid MEXC secret: {err}")))?;
    mac.update(query.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}
