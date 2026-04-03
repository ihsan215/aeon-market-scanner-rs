use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct MexcAccountInfoResponse {
    #[serde(rename = "accountType")]
    pub account_type: Option<String>,
    pub balances: Vec<MexcBalanceEntry>,
    pub permissions: Option<Vec<String>>,
    #[serde(rename = "canTrade")]
    pub can_trade: Option<bool>,
    #[serde(rename = "canWithdraw")]
    pub can_withdraw: Option<bool>,
    #[serde(rename = "canDeposit")]
    pub can_deposit: Option<bool>,
    #[serde(rename = "updateTime")]
    pub update_time: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MexcBalanceEntry {
    pub asset: String,
    pub free: String,
    pub locked: String,
    pub available: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct MexcListenKeyResponse {
    #[serde(rename = "listenKey")]
    pub listen_key: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MexcPrivateOrdersEnvelope {
    pub channel: String,
    #[serde(default)]
    pub symbol: String,
    #[serde(rename = "sendTime")]
    pub send_time: Option<u64>,
    #[serde(rename = "privateOrders")]
    pub private_orders: MexcPrivateOrderPayload,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct MexcPrivateOrderPayload {
    #[serde(rename = "orderId", alias = "id", default)]
    pub order_id: Option<String>,
    #[serde(rename = "clientId", alias = "c")]
    pub client_id: Option<String>,
    #[serde(alias = "p")]
    pub price: String,
    #[serde(alias = "v")]
    pub quantity: String,
    #[serde(alias = "a")]
    pub amount: String,
    #[serde(rename = "avgPrice", alias = "ap", default)]
    pub avg_price: String,
    #[serde(rename = "orderType", alias = "ot")]
    pub order_type: i32,
    #[serde(rename = "tradeType", alias = "S")]
    pub trade_type: i32,
    #[serde(rename = "remainAmount", alias = "A")]
    pub remain_amount: String,
    #[serde(rename = "remainQuantity", alias = "V")]
    pub remain_quantity: String,
    #[serde(rename = "lastDealQuantity")]
    pub last_deal_quantity: Option<String>,
    #[serde(rename = "cumulativeQuantity", alias = "cv", default)]
    pub cumulative_quantity: String,
    #[serde(rename = "cumulativeAmount", alias = "ca", default)]
    pub cumulative_amount: String,
    pub status: i32,
    #[serde(rename = "createTime", alias = "t")]
    pub create_time: Option<u64>,
}
