mod types;

use crate::cex::gateio::types::GateioOrderBookResponse;
use crate::common::{
    CEXTrait, CexExchange, CexPrice, Exchange, ExchangeTrait, MarketScannerError, find_mid_price,
    format_symbol_for_exchange, get_timestamp_millis, parse_f64,
};
use crate::create_exchange;
use async_trait::async_trait;

const GATEIO_API_BASE: &str = "https://api.gateio.ws/api/v4";

create_exchange!(Gateio);

#[async_trait]
impl ExchangeTrait for Gateio {
    fn api_base(&self) -> &str {
        GATEIO_API_BASE
    }

    fn client(&self) -> &reqwest::Client {
        &self.client
    }

    fn exchange_name(&self) -> &str {
        "Gate.io"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        // Gate.io time endpoint - test connectivity to the REST API
        let endpoint = "spot/time";
        self.get::<serde_json::Value>(endpoint)
            .await
            .map_err(|_| MarketScannerError::HealthCheckFailed)?;

        Ok(())
    }
}

#[async_trait]
impl CEXTrait for Gateio {
    fn supports_websocket(&self) -> bool {
        false
    }

    async fn get_price(&self, symbol: &str) -> Result<CexPrice, MarketScannerError> {
        // Validate symbol is not empty
        if symbol.is_empty() {
            return Err(MarketScannerError::InvalidSymbol(
                "Symbol cannot be empty".to_string(),
            ));
        }

        // Format symbol for Gate.io
        let gateio_symbol = format_symbol_for_exchange(symbol, &CexExchange::Gateio)?;

        // Get order book for bid/ask prices and quantities (limit=1 for best bid/ask only)
        let book_endpoint = format!("spot/order_book?currency_pair={}&limit=1", gateio_symbol);
        let order_book: GateioOrderBookResponse = self.get(&book_endpoint).await?;

        // Get best bid (first element in bids array: [price, quantity])
        let bid_entry = order_book.bids.first().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("No bid found for symbol: {}", symbol))
        })?;

        // Get best ask (first element in asks array: [price, quantity])
        let ask_entry = order_book.asks.first().ok_or_else(|| {
            MarketScannerError::InvalidSymbol(format!("No ask found for symbol: {}", symbol))
        })?;

        let bid = parse_f64(&bid_entry[0], "bid price")?;
        let ask = parse_f64(&ask_entry[0], "ask price")?;
        let bid_qty = parse_f64(&bid_entry[1], "bid quantity")?;
        let ask_qty = parse_f64(&ask_entry[1], "ask quantity")?;

        let mid_price = find_mid_price(bid, ask);

        // Convert Gate.io symbol format (BTC_USDT) back to standard (BTCUSDT)
        let standard_symbol = gateio_symbol.replace("_", "");

        Ok(CexPrice {
            symbol: standard_symbol,
            mid_price,
            bid_price: bid,
            ask_price: ask,
            bid_qty,
            ask_qty,
            timestamp: get_timestamp_millis(),
            exchange: Exchange::Cex(CexExchange::Gateio),
        })
    }
}
