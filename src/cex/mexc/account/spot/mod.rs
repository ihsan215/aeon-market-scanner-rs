mod types;

use crate::common::{CexExchange, MarketScannerError, format_symbol_for_exchange};
use reqwest::Method;
use std::collections::BTreeMap;

pub use types::{MexcCancelledOrder, MexcLimitOrderType, MexcPlacedOrder};

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
}

fn trade_side_to_api_value(side: super::stream::types::MexcTradeSide) -> &'static str {
    match side {
        super::stream::types::MexcTradeSide::Buy => "BUY",
        super::stream::types::MexcTradeSide::Sell => "SELL",
        super::stream::types::MexcTradeSide::Unknown(_) => "BUY",
    }
}
