const DEFAULT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

pub fn create_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(DEFAULT_TIMEOUT)
        .build()
        .expect("Failed to create HTTP client")
}
