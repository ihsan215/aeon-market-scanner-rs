use serde::Deserialize;

// MEXC protobuf: PublicAggreBookTickerV3Api (field 315 in wrapper)
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MexcAggreBookTicker {
    #[prost(string, tag = "1")]
    pub bid_price: String,
    #[prost(string, tag = "2")]
    pub bid_quantity: String,
    #[prost(string, tag = "3")]
    pub ask_price: String,
    #[prost(string, tag = "4")]
    pub ask_quantity: String,
}

#[derive(Clone, PartialEq, ::prost::Oneof)]
pub enum MexcPushBody {
    #[prost(message, tag = "304")]
    PrivateOrders(MexcPrivateOrdersPb),
    #[prost(message, tag = "315")]
    PublicAggreBookTicker(MexcAggreBookTicker),
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MexcPushDataWrapper {
    #[prost(string, tag = "1")]
    pub channel: String,
    #[prost(oneof = "MexcPushBody", tags = "304, 315")]
    pub body: Option<MexcPushBody>,
    #[prost(string, optional, tag = "3")]
    pub symbol: Option<String>,
    #[prost(string, optional, tag = "4")]
    pub symbol_id: Option<String>,
    #[prost(int64, optional, tag = "5")]
    pub create_time: Option<i64>,
    #[prost(int64, optional, tag = "6")]
    pub send_time: Option<i64>,
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MexcPrivateOrdersPb {
    #[prost(string, tag = "1")]
    pub id: String,
    #[prost(string, tag = "2")]
    pub client_id: String,
    #[prost(string, tag = "3")]
    pub price: String,
    #[prost(string, tag = "4")]
    pub quantity: String,
    #[prost(string, tag = "5")]
    pub amount: String,
    #[prost(string, tag = "6")]
    pub avg_price: String,
    #[prost(int32, tag = "7")]
    pub order_type: i32,
    #[prost(int32, tag = "8")]
    pub trade_type: i32,
    #[prost(bool, tag = "9")]
    pub is_maker: bool,
    #[prost(string, tag = "10")]
    pub remain_amount: String,
    #[prost(string, tag = "11")]
    pub remain_quantity: String,
    #[prost(string, optional, tag = "12")]
    pub last_deal_quantity: Option<String>,
    #[prost(string, tag = "13")]
    pub cumulative_quantity: String,
    #[prost(string, tag = "14")]
    pub cumulative_amount: String,
    #[prost(int32, tag = "15")]
    pub status: i32,
    #[prost(int64, tag = "16")]
    pub create_time: i64,
    #[prost(string, optional, tag = "17")]
    pub market: Option<String>,
    #[prost(int32, optional, tag = "18")]
    pub trigger_type: Option<i32>,
    #[prost(string, optional, tag = "19")]
    pub trigger_price: Option<String>,
    #[prost(int32, optional, tag = "20")]
    pub state: Option<i32>,
    #[prost(string, optional, tag = "21")]
    pub oco_id: Option<String>,
    #[prost(string, optional, tag = "22")]
    pub route_factor: Option<String>,
    #[prost(string, optional, tag = "23")]
    pub symbol_id: Option<String>,
    #[prost(string, optional, tag = "24")]
    pub market_id: Option<String>,
    #[prost(string, optional, tag = "25")]
    pub market_currency_id: Option<String>,
    #[prost(string, optional, tag = "26")]
    pub currency_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct MexcBookTickerResponse {
    pub symbol: String,
    #[serde(rename = "bidPrice")]
    pub bid_price: String,
    #[serde(rename = "bidQty")]
    pub bid_qty: String,
    #[serde(rename = "askPrice")]
    pub ask_price: String,
    #[serde(rename = "askQty")]
    pub ask_qty: String,
}
