use serde::{Deserialize, Serialize};

use super::super::stream::types::MexcTradeSide;
use crate::common::{CexExchange, MarketScannerError, format_symbol_for_exchange};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MexcLimitOrderType {
    Limit,
    PostOnly,
    ImmediateOrCancel,
    FillOrKill,
}

impl MexcLimitOrderType {
    pub fn api_value(&self) -> &'static str {
        match self {
            Self::Limit => "LIMIT",
            Self::PostOnly => "LIMIT_MAKER",
            Self::ImmediateOrCancel => "IMMEDIATE_OR_CANCEL",
            Self::FillOrKill => "FILL_OR_KILL",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct MexcPlacedOrder {
    pub symbol: String,
    #[serde(
        rename = "orderId",
        deserialize_with = "deserialize_string_from_anything"
    )]
    pub order_id: String,
    #[serde(rename = "orderListId")]
    pub order_list_id: i64,
    pub price: String,
    #[serde(rename = "origQty", default)]
    pub orig_qty: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    #[serde(rename = "stpMode", default)]
    pub stp_mode: String,
    #[serde(rename = "transactTime")]
    pub transact_time: u64,
}

/// One entry in [`super::Mexc::place_batch_orders`] `batchOrders` JSON (max 20 per request, same symbol).
#[derive(Debug, Clone, Serialize)]
pub struct MexcBatchOrderItem {
    pub symbol: String,
    pub side: String,
    #[serde(rename = "type")]
    pub order_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quantity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price: Option<String>,
    #[serde(rename = "quoteOrderQty", skip_serializing_if = "Option::is_none")]
    pub quote_order_qty: Option<String>,
    #[serde(rename = "newClientOrderId", skip_serializing_if = "Option::is_none")]
    pub new_client_order_id: Option<String>,
    #[serde(rename = "stpMode", skip_serializing_if = "Option::is_none")]
    pub stp_mode: Option<String>,
}

impl MexcBatchOrderItem {
    pub fn limit(
        symbol: &str,
        side: MexcTradeSide,
        price: &str,
        quantity: &str,
        order_type: MexcLimitOrderType,
    ) -> Result<Self, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        Ok(Self {
            symbol: mexc_symbol,
            side: trade_side_to_api_string(side),
            order_type: order_type.api_value().to_string(),
            quantity: Some(quantity.to_string()),
            price: Some(price.to_string()),
            quote_order_qty: None,
            new_client_order_id: None,
            stp_mode: None,
        })
    }

    pub fn market_by_quantity(
        symbol: &str,
        side: MexcTradeSide,
        quantity: &str,
    ) -> Result<Self, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        Ok(Self {
            symbol: mexc_symbol,
            side: trade_side_to_api_string(side),
            order_type: "MARKET".to_string(),
            quantity: Some(quantity.to_string()),
            price: None,
            quote_order_qty: None,
            new_client_order_id: None,
            stp_mode: None,
        })
    }

    pub fn market_by_quote_amount(
        symbol: &str,
        side: MexcTradeSide,
        quote_order_qty: &str,
    ) -> Result<Self, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        Ok(Self {
            symbol: mexc_symbol,
            side: trade_side_to_api_string(side),
            order_type: "MARKET".to_string(),
            quantity: None,
            price: None,
            quote_order_qty: Some(quote_order_qty.to_string()),
            new_client_order_id: None,
            stp_mode: None,
        })
    }
}

fn trade_side_to_api_string(side: MexcTradeSide) -> String {
    match side {
        MexcTradeSide::Buy => "BUY".to_string(),
        MexcTradeSide::Sell => "SELL".to_string(),
        MexcTradeSide::Unknown(_) => "BUY".to_string(),
    }
}

/// One row in the batch response: success rows have `order_id`; failures may set `code` / `msg` instead.
#[derive(Debug, Clone, Deserialize)]
pub struct MexcBatchOrderResult {
    #[serde(default)]
    pub symbol: Option<String>,
    #[serde(
        rename = "orderId",
        default,
        deserialize_with = "deserialize_optional_string_from_anything"
    )]
    pub order_id: Option<String>,
    #[serde(rename = "orderListId", default)]
    pub order_list_id: Option<i64>,
    #[serde(
        rename = "newClientOrderId",
        default,
        deserialize_with = "deserialize_optional_string_from_anything"
    )]
    pub new_client_order_id: Option<String>,
    #[serde(default)]
    pub msg: Option<String>,
    #[serde(default)]
    pub code: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MexcCancelledOrder {
    pub symbol: String,
    #[serde(
        rename = "origClientOrderId",
        default,
        deserialize_with = "deserialize_optional_string_from_anything"
    )]
    pub orig_client_order_id: Option<String>,
    #[serde(
        rename = "orderId",
        deserialize_with = "deserialize_string_from_anything"
    )]
    pub order_id: String,
    #[serde(
        rename = "clientOrderId",
        default,
        deserialize_with = "deserialize_optional_string_from_anything"
    )]
    pub client_order_id: Option<String>,
    pub price: String,
    #[serde(rename = "origQty", default)]
    pub orig_qty: String,
    #[serde(rename = "executedQty", default)]
    pub executed_qty: String,
    #[serde(rename = "cummulativeQuoteQty", default)]
    pub cumulative_quote_qty: String,
    pub status: String,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
}

fn deserialize_string_from_anything<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::String(v) => Ok(v),
        serde_json::Value::Number(v) => Ok(v.to_string()),
        serde_json::Value::Null => Ok(String::new()),
        other => Err(serde::de::Error::custom(format!(
            "unexpected value for string field: {other}"
        ))),
    }
}

fn deserialize_optional_string_from_anything<'de, D>(
    deserializer: D,
) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        Some(serde_json::Value::String(v)) => Ok(Some(v)),
        Some(serde_json::Value::Number(v)) => Ok(Some(v.to_string())),
        Some(serde_json::Value::Null) | None => Ok(None),
        Some(other) => Err(serde::de::Error::custom(format!(
            "unexpected value for optional string field: {other}"
        ))),
    }
}
