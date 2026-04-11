mod types;

use crate::common::{CexExchange, MarketScannerError, format_symbol_for_exchange};
use reqwest::Method;
use std::collections::BTreeMap;

pub use types::{
    MexcBatchOrderItem, MexcBatchOrderResult, MexcCancelledOrder, MexcLimitOrderType,
    MexcPlacedOrder,
};

impl super::super::Mexc {
    pub async fn place_market_order_by_quantity(
        &self,
        symbol: &str,
        side: super::stream::types::MexcTradeSide,
        quantity: &str,
    ) -> Result<MexcPlacedOrder, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), mexc_symbol);
        params.insert(
            "side".to_string(),
            trade_side_to_api_value(side).to_string(),
        );
        params.insert("type".to_string(), "MARKET".to_string());
        params.insert("quantity".to_string(), quantity.to_string());

        super::auth::signed_request(self, Method::POST, "order", params).await
    }

    pub async fn place_market_order_by_quote_amount(
        &self,
        symbol: &str,
        side: super::stream::types::MexcTradeSide,
        quote_order_qty: &str,
    ) -> Result<MexcPlacedOrder, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), mexc_symbol);
        params.insert(
            "side".to_string(),
            trade_side_to_api_value(side).to_string(),
        );
        params.insert("type".to_string(), "MARKET".to_string());
        params.insert("quoteOrderQty".to_string(), quote_order_qty.to_string());

        super::auth::signed_request(self, Method::POST, "order", params).await
    }

    pub async fn place_limit_order(
        &self,
        symbol: &str,
        side: super::stream::types::MexcTradeSide,
        price: &str,
        quantity: &str,
        order_type: MexcLimitOrderType,
    ) -> Result<MexcPlacedOrder, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), mexc_symbol);
        params.insert(
            "side".to_string(),
            trade_side_to_api_value(side).to_string(),
        );
        params.insert("type".to_string(), order_type.api_value().to_string());
        params.insert("price".to_string(), price.to_string());
        params.insert("quantity".to_string(), quantity.to_string());

        super::auth::signed_request(self, Method::POST, "order", params).await
    }

    pub async fn cancel_order(
        &self,
        symbol: &str,
        order_id: &str,
    ) -> Result<MexcCancelledOrder, MarketScannerError> {
        let mexc_symbol = format_symbol_for_exchange(symbol, &CexExchange::MEXC)?;
        let mut params = BTreeMap::new();
        params.insert("symbol".to_string(), mexc_symbol);
        params.insert("orderId".to_string(), order_id.to_string());

        super::auth::signed_request(self, Method::DELETE, "order", params).await
    }

    /// Batch place up to 20 orders for the **same** symbol ([MEXC batch orders](https://www.mexc.com/api-docs/spot-v3/spot-account-trade#batch-orders)).
    pub async fn place_batch_orders(
        &self,
        orders: &[MexcBatchOrderItem],
    ) -> Result<Vec<MexcBatchOrderResult>, MarketScannerError> {
        if orders.is_empty() {
            return Err(MarketScannerError::ApiError(
                "MEXC batchOrders requires at least one order".to_string(),
            ));
        }
        if orders.len() > 20 {
            return Err(MarketScannerError::ApiError(
                "MEXC batchOrders supports at most 20 orders per request".to_string(),
            ));
        }

        let batch_json = serde_json::to_string(orders).map_err(|e| {
            MarketScannerError::ApiError(format!("batchOrders JSON serialization failed: {e}"))
        })?;
        let mut params = BTreeMap::new();
        params.insert("batchOrders".to_string(), batch_json);

        super::auth::signed_request(self, Method::POST, "batchOrders", params).await
    }

    /// Convenience: batch limit orders on one symbol (same constraints as [`Self::place_batch_orders`]).
    pub async fn place_batch_limit_orders(
        &self,
        symbol: &str,
        legs: &[(
            super::stream::types::MexcTradeSide,
            &str,
            &str,
            MexcLimitOrderType,
        )],
    ) -> Result<Vec<MexcBatchOrderResult>, MarketScannerError> {
        let mut items = Vec::with_capacity(legs.len());
        for (side, price, quantity, order_type) in legs {
            items.push(MexcBatchOrderItem::limit(
                symbol,
                *side,
                price,
                quantity,
                *order_type,
            )?);
        }
        self.place_batch_orders(&items).await
    }
}

fn trade_side_to_api_value(side: super::stream::types::MexcTradeSide) -> &'static str {
    match side {
        super::stream::types::MexcTradeSide::Buy => "BUY",
        super::stream::types::MexcTradeSide::Sell => "SELL",
        super::stream::types::MexcTradeSide::Unknown(_) => "BUY",
    }
}
