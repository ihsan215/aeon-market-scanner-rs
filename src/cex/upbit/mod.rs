mod types;

use crate::cex::upbit::types::UpbitOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, format_symbol_for_exchange_ws, get_timestamp_millis,
    normalize_symbol, standard_symbol_for_cex_ws_response,
};
use crate::create_exchange;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message as WsMessage;

const UPBIT_API_BASE: &str = "https://api.upbit.com/v1";
const UPBIT_WS_URL: &str = "wss://api.upbit.com/websocket/v1";

create_exchange!(Upbit);

#[async_trait]
impl ExchangeTrait for Upbit {
    fn api_base(&self) -> &str {
        UPBIT_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Upbit"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Upbit market all endpoint - test connectivity to the REST API
        let endpoint = "market/all?isDetails=false";
        let response: serde_json::Value = self.get(endpoint).await?;

        // Upbit returns array of market objects for success
        if let Some(array) = response.as_array() {
            if !array.is_empty() {
                return Ok(());
            }
        }

        Err(MarketScannerError::HealthCheckFailed)
    }
}

#[async_trait]
impl CEXTrait for Upbit {
    fn supports_websocket(&self) -> bool {
        true
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Upbit (KRW-BTC format)
        let upbit_symbol = format_symbol_for_exchange(symbol, &CexExchange::Upbit)?;

        // Using orderbook endpoint
        let endpoint = format!("orderbook?markets={}", upbit_symbol);

        // First get as JSON value to handle errors gracefully
        let response: serde_json::Value = self.get(&endpoint).await?;

        // Check if response is an error
        if let Some(error) = response.get("error") {
            let error_msg = error.as_str().unwrap_or("Unknown error");
            return Err(MarketScannerError::ApiError(format!(
                "Upbit API error: {}",
                error_msg
            )));
        }

        // Deserialize response to UpbitOrderBookResponse (it's an array with one element)
        let orderbook_array = response.as_array().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: invalid orderbook response format for symbol: {}",
                symbol
            ))
        })?;

        let orderbook_response: UpbitOrderBookResponse = serde_json::from_value(
            orderbook_array
                .first()
                .ok_or_else(|| {
                    MarketScannerError::ApiError(format!(
                        "Upbit API error: empty orderbook response for symbol: {}",
                        symbol
                    ))
                })?
                .clone(),
        )
        .map_err(|e| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: failed to parse orderbook response: {}",
                e
            ))
        })?;

        // Get best bid and ask from first orderbook unit
        let best_unit = orderbook_response.orderbook_units.first().ok_or_else(|| {
            MarketScannerError::ApiError(format!(
                "Upbit API error: no orderbook units found for symbol: {}",
                symbol
            ))
        })?;

        let bid = best_unit.bid_price;
        let ask = best_unit.ask_price;
        let bid_qty = best_unit.bid_size;
        let ask_qty = best_unit.ask_size;

        // Ensure bid <= ask
        let (bid, ask, bid_qty, ask_qty) = if bid > ask {
            (ask, bid, ask_qty, bid_qty)
        } else {
            (bid, ask, bid_qty, ask_qty)
        };

        let mid_price = find_mid_price(bid, ask);

        // Normalize symbol back to standard format
        let standard_symbol = normalize_symbol(symbol);

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Upbit),
        })
    }

    async fn stream_price_websocket(
        &self,
        symbols: &[&str],
        reconnect_attempts: u32,
        reconnect_delay_ms: u64,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        if symbols.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "At least one symbol required".to_string(),
            ));
        }

        let upbit_symbols: Vec<String> = symbols
            .iter()
            .map(|s| format_symbol_for_exchange_ws(s, &CexExchange::Upbit))
            .collect::<Result<Vec<_>, _>>()?;

        // Subscribe: [{ticket},{type,codes},{format}]
        let subscribe_msg = serde_json::json!([
            {"ticket": "upbit-ws-1"},
            {"type": "orderbook", "codes": upbit_symbols},
            {"format": "DEFAULT"}
        ]);

        let (tx, rx) = mpsc::channel(64);
        let delay =
            std::time::Duration::from_millis(if reconnect_delay_ms == 0 { 1000 } else { reconnect_delay_ms });

        tokio::spawn(async move {
            let mut attempt = 0u32;
            loop {
                attempt += 1;
                let (mut ws_stream, _) = match tokio_tungstenite::connect_async(UPBIT_WS_URL).await
                {
                    Ok(v) => v,
                    Err(_) => {
                        if tx.is_closed()
                            || reconnect_attempts == 0
                            || attempt > reconnect_attempts
                        {
                            break;
                        }
                        tokio::time::sleep(delay).await;
                        continue;
                    }
                };

                if ws_stream
                    .send(WsMessage::Text(subscribe_msg.to_string()))
                    .await
                    .is_err()
                {
                    if tx.is_closed()
                        || reconnect_attempts == 0
                        || attempt > reconnect_attempts
                    {
                        break;
                    }
                    tokio::time::sleep(delay).await;
                    continue;
                }

                let (_write, mut read) = ws_stream.split();

                while let Some(Ok(msg)) = read.next().await {
                    let text = match msg.into_text() {
                        Ok(t) => t,
                        Err(_) => continue,
                    };
                    let value: serde_json::Value = match serde_json::from_str(&text) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if value.get("type").and_then(|t| t.as_str()) != Some("orderbook") {
                        continue;
                    }
                    if let Some(price) = parse_upbit_orderbook(&value) {
                        if tx.send(price).await.is_err() {
                            return;
                        }
                    }
                }

                if tx.is_closed()
                    || reconnect_attempts == 0
                    || attempt > reconnect_attempts
                {
                    break;
                }
                tokio::time::sleep(delay).await;
            }
        });

        Ok(rx)
    }
}

fn parse_upbit_orderbook(value: &serde_json::Value) -> Option<CexPrice> {
    let code = value.get("code")?.as_str()?;
    let orderbook_units = value.get("orderbook_units")?.as_array()?;
    let unit = orderbook_units.first()?.as_object()?;

    let bid_price = unit.get("bid_price")?.as_f64()?;
    let ask_price = unit.get("ask_price")?.as_f64()?;
    let bid_size = unit.get("bid_size").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let ask_size = unit.get("ask_size").and_then(|v| v.as_f64()).unwrap_or(0.0);

    if bid_price <= 0.0 || ask_price <= 0.0 {
        return None;
    }

    let standard_symbol = standard_symbol_for_cex_ws_response(code, &CexExchange::Upbit);

    Some(CexPrice {
        symbol: standard_symbol,
        mid_price: find_mid_price(bid_price, ask_price),
        bid_price: bid_price,
        ask_price: ask_price,
        bid_qty: bid_size,
        ask_qty: ask_size,
        timestamp: get_timestamp_millis(),
        exchange: Exchange::Cex(CexExchange::Upbit),
    })
}
