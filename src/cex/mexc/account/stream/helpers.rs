use crate::common::{MarketScannerError, normalize_symbol, parse_f64};
use prost::Message;

pub(super) fn parse_private_order_update(
    event: super::super::types::MexcPrivateOrdersEnvelope,
) -> Result<super::types::MexcSpotOrderUpdate, MarketScannerError> {
    let payload = event.private_orders;

    Ok(super::types::MexcSpotOrderUpdate {
        symbol: normalize_symbol(&event.symbol),
        channel: event.channel,
        send_time: event.send_time.unwrap_or_default(),
        order_id: payload.order_id.unwrap_or_default(),
        client_id: payload.client_id.unwrap_or_default(),
        price: parse_f64(&payload.price, "mexc order price")?,
        quantity: parse_f64(&payload.quantity, "mexc order quantity")?,
        amount: parse_f64(&payload.amount, "mexc order amount")?,
        avg_price: parse_f64(&payload.avg_price, "mexc avg price").unwrap_or(0.0),
        order_type: payload.order_type.into(),
        side: payload.trade_type.into(),
        remain_amount: parse_f64(&payload.remain_amount, "mexc remain amount")?,
        remain_quantity: parse_f64(&payload.remain_quantity, "mexc remain quantity")?,
        last_deal_quantity: payload
            .last_deal_quantity
            .as_deref()
            .map(|value| parse_f64(value, "mexc last deal quantity"))
            .transpose()?,
        cumulative_quantity: parse_f64(&payload.cumulative_quantity, "mexc cumulative quantity")?,
        cumulative_amount: parse_f64(&payload.cumulative_amount, "mexc cumulative amount")?,
        status: payload.status.into(),
        create_time: payload.create_time.unwrap_or_default(),
    })
}

pub(super) fn parse_private_order_update_from_binary(
    bytes: &[u8],
) -> Result<Option<super::types::MexcSpotOrderUpdate>, MarketScannerError> {
    let wrapper = super::super::super::types::MexcPushDataWrapper::decode(
        prost::bytes::Bytes::copy_from_slice(bytes),
    )
    .map_err(|err| MarketScannerError::WsRpcError(format!("Mexc protobuf decode error: {err}")))?;

    let body = match wrapper.body {
        Some(body) => body,
        None => return Ok(None),
    };

    let order = match body {
        super::super::super::types::MexcPushBody::PrivateOrders(order) => order,
        _ => return Ok(None),
    };

    let symbol = wrapper
        .symbol
        .as_deref()
        .filter(|s| !s.is_empty())
        .or(order.market.as_deref())
        .unwrap_or("");

    Ok(Some(super::types::MexcSpotOrderUpdate {
        symbol: normalize_symbol(symbol),
        channel: wrapper.channel,
        send_time: wrapper.send_time.unwrap_or_default() as u64,
        order_id: order.id,
        client_id: order.client_id,
        price: parse_f64(&order.price, "mexc order price")?,
        quantity: parse_f64(&order.quantity, "mexc order quantity")?,
        amount: parse_f64(&order.amount, "mexc order amount")?,
        avg_price: parse_f64(&order.avg_price, "mexc avg price").unwrap_or(0.0),
        order_type: order.order_type.into(),
        side: order.trade_type.into(),
        remain_amount: parse_f64(&order.remain_amount, "mexc remain amount")?,
        remain_quantity: parse_f64(&order.remain_quantity, "mexc remain quantity")?,
        last_deal_quantity: order
            .last_deal_quantity
            .as_deref()
            .map(|value| parse_f64(value, "mexc last deal quantity"))
            .transpose()?,
        cumulative_quantity: parse_f64(&order.cumulative_quantity, "mexc cumulative quantity")?,
        cumulative_amount: parse_f64(&order.cumulative_amount, "mexc cumulative amount")?,
        status: order.status.into(),
        create_time: order.create_time.max(0) as u64,
    }))
}
