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
    #[prost(message, tag = "315")]
    PublicAggreBookTicker(MexcAggreBookTicker),
}

#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MexcPushDataWrapper {
    #[prost(string, tag = "1")]
    pub channel: String,
    #[prost(oneof = "MexcPushBody", tags = "315")]
    pub body: Option<MexcPushBody>,
    #[prost(string, optional, tag = "3")]
    pub symbol: Option<String>,
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
