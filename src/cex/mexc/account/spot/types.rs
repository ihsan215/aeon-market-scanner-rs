use serde::Deserialize;

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
