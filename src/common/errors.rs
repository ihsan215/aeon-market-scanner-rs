#[derive(thiserror::Error, Debug)]
pub enum MarketScannerError {
    #[error("Health check failed")]
    HealthCheckFailed,

    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(#[from] serde_json::Error),

    #[error("Invalid symbol: {0}")]
    InvalidSymbol(String),

    #[error("WebSocket / RPC error: {0}")]
    WsRpcError(String),
}
