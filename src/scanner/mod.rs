use crate::common::{
    CEXTrait, CexExchange, CexPrice, DEXTrait, DexAggregator, DexPrice, MarketScannerError,
};
use crate::dex::chains::Token;
use crate::{
    Binance, Bitfinex, Bitget, Btcturk, Bybit, Coinbase, Cryptocom, Gateio, Htx, Kraken, Kucoin,
    KyberSwap, Mexc, OKX, Upbit,
};
use futures::future::join_all;

mod opportunity;
pub use opportunity::{ArbitrageOpportunity, PriceData};

/// Arbitrage scanner - fetches price data from CEX and DEX exchanges and finds arbitrage opportunities
pub struct ArbitrageScanner;

impl ArbitrageScanner {
    /// Fetches price data from CEX and DEX exchanges and finds arbitrage opportunities, sorted by profitability
    ///
    /// # Arguments
    /// * `symbol` - Symbol to scan (e.g., "BTCUSDT")
    /// * `cex_exchanges` - List of CEX exchanges
    /// * `dex_exchanges` - List of DEX exchanges (optional)
    /// * `base_token` - Base token for DEX (optional, required if DEX is used)
    /// * `quote_token` - Quote token for DEX (optional, required if DEX is used)
    /// * `quote_amount` - Quote amount for DEX (optional, required if DEX is used)
    ///
    /// # Returns
    /// List of arbitrage opportunities sorted by profitability (most profitable first)
    /// Each opportunity contains full response data from get_price calls (timestamp, route data, etc.)
    pub async fn scan_arbitrage_opportunities(
        symbol: &str,
        cex_exchanges: &[CexExchange],
        dex_exchanges: Option<&[DexAggregator]>,
        base_token: Option<&Token>,
        quote_token: Option<&Token>,
        quote_amount: Option<f64>,
    ) -> Result<Vec<ArbitrageOpportunity>, MarketScannerError> {
        // Fetch all prices in parallel
        let (cex_prices, dex_prices) = tokio::try_join!(
            Self::fetch_cex_prices(cex_exchanges, symbol),
            Self::fetch_dex_prices(dex_exchanges, base_token, quote_token, quote_amount)
        )?;

        // Find arbitrage opportunities by matching buy and sell candidates
        let opportunities = Self::find_opportunities(&cex_prices, &dex_prices);

        // Sort by profitability (most profitable first)
        let mut opportunities = opportunities;
        opportunities.sort_by(|a, b| {
            b.profit_percentage
                .partial_cmp(&a.profit_percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(opportunities)
    }

    /// Fetches CEX prices in parallel
    async fn fetch_cex_prices(
        exchanges: &[CexExchange],
        symbol: &str,
    ) -> Result<Vec<CexPrice>, MarketScannerError> {
        let futures: Vec<_> = exchanges
            .iter()
            .map(|exchange| Self::get_cex_price(exchange, symbol))
            .collect();

        let results = join_all(futures).await;
        let mut prices = Vec::new();

        for (exchange, result) in exchanges.iter().zip(results) {
            match result {
                Ok(price) => prices.push(price),
                Err(e) => {
                    eprintln!("Warning: Failed to get price from {:?}: {:?}", exchange, e);
                }
            }
        }

        Ok(prices)
    }

    /// Fetches DEX prices in parallel
    async fn fetch_dex_prices(
        exchanges: Option<&[DexAggregator]>,
        base_token: Option<&Token>,
        quote_token: Option<&Token>,
        quote_amount: Option<f64>,
    ) -> Result<Vec<DexPrice>, MarketScannerError> {
        let mut prices = Vec::new();

        if let Some(dex_list) = exchanges {
            if let (Some(base), Some(quote), Some(amount)) = (base_token, quote_token, quote_amount) {
                let futures: Vec<_> = dex_list
                    .iter()
                    .map(|exchange| Self::get_dex_price(exchange, base, quote, amount))
                    .collect();

                let results = join_all(futures).await;
                for (exchange, result) in dex_list.iter().zip(results) {
                    match result {
                        Ok(price) => prices.push(price),
                        Err(e) => {
                            eprintln!("Warning: Failed to get price from {:?}: {:?}", exchange, e);
                        }
                    }
                }
            }
        }

        Ok(prices)
    }

    /// Finds arbitrage opportunities by matching buy and sell candidates
    fn find_opportunities(
        cex_prices: &[CexPrice],
        dex_prices: &[DexPrice],
    ) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        // Create buy candidates (sorted by ask price - lowest first)
        let mut buy_candidates = Vec::new();
        for cex_price in cex_prices {
            buy_candidates.push((
                cex_price.ask_price,
                PriceData::Cex(cex_price.clone()),
                Self::exchange_name(&cex_price.exchange),
            ));
        }
        for dex_price in dex_prices {
            buy_candidates.push((
                dex_price.ask_price,
                PriceData::Dex(dex_price.clone()),
                Self::exchange_name(&dex_price.exchange),
            ));
        }
        buy_candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Create sell candidates (sorted by bid price - highest first)
        let mut sell_candidates = Vec::new();
        for cex_price in cex_prices {
            sell_candidates.push((
                cex_price.bid_price,
                PriceData::Cex(cex_price.clone()),
                Self::exchange_name(&cex_price.exchange),
            ));
        }
        for dex_price in dex_prices {
            sell_candidates.push((
                dex_price.bid_price,
                PriceData::Dex(dex_price.clone()),
                Self::exchange_name(&dex_price.exchange),
            ));
        }
        sell_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Match buy and sell candidates
        for (buy_price, buy_data, buy_exchange) in &buy_candidates {
            for (sell_price, sell_data, sell_exchange) in &sell_candidates {
                if buy_exchange == sell_exchange || *sell_price <= *buy_price {
                    continue;
                }

                let profit = sell_price - buy_price;
                let profit_percentage = (profit / buy_price) * 100.0;

                if profit_percentage < 0.01 {
                    continue;
                }

                let (symbol, buy_qty, sell_qty) = Self::extract_quantities(buy_data, sell_data);

                opportunities.push(ArbitrageOpportunity {
                    buy_exchange: buy_exchange.clone(),
                    sell_exchange: sell_exchange.clone(),
                    symbol,
                    buy_price: *buy_price,
                    sell_price: *sell_price,
                    profit,
                    profit_percentage,
                    buy_quantity: buy_qty.min(sell_qty),
                    sell_quantity: sell_qty.min(buy_qty),
                    buy_price_data: buy_data.clone(), // Contains full response: timestamp, route data, etc.
                    sell_price_data: sell_data.clone(), // Contains full response: timestamp, route data, etc.
                });
            }
        }

        opportunities
    }

    /// Extracts symbol and quantities from price data
    fn extract_quantities(buy_data: &PriceData, sell_data: &PriceData) -> (String, f64, f64) {
        match (buy_data, sell_data) {
            (PriceData::Cex(cex_buy), PriceData::Cex(cex_sell)) => {
                (cex_buy.symbol.clone(), cex_buy.ask_qty, cex_sell.bid_qty)
            }
            (PriceData::Cex(cex_buy), PriceData::Dex(dex_sell)) => {
                (cex_buy.symbol.clone(), cex_buy.ask_qty, dex_sell.bid_qty)
            }
            (PriceData::Dex(dex_buy), PriceData::Cex(cex_sell)) => {
                (dex_buy.symbol.clone(), dex_buy.ask_qty, cex_sell.bid_qty)
            }
            (PriceData::Dex(dex_buy), PriceData::Dex(dex_sell)) => {
                (dex_buy.symbol.clone(), dex_buy.ask_qty, dex_sell.bid_qty)
            }
        }
    }

    /// Gets price from a CEX exchange
    async fn get_cex_price(
        exchange: &CexExchange,
        symbol: &str,
    ) -> Result<CexPrice, MarketScannerError> {
        match exchange {
            CexExchange::Binance => Binance::new().get_price(symbol).await,
            CexExchange::Bybit => Bybit::new().get_price(symbol).await,
            CexExchange::MEXC => Mexc::new().get_price(symbol).await,
            CexExchange::OKX => OKX::new().get_price(symbol).await,
            CexExchange::Gateio => Gateio::new().get_price(symbol).await,
            CexExchange::Kucoin => Kucoin::new().get_price(symbol).await,
            CexExchange::Bitget => Bitget::new().get_price(symbol).await,
            CexExchange::Btcturk => Btcturk::new().get_price(symbol).await,
            CexExchange::Htx => Htx::new().get_price(symbol).await,
            CexExchange::Coinbase => Coinbase::new().get_price(symbol).await,
            CexExchange::Kraken => Kraken::new().get_price(symbol).await,
            CexExchange::Bitfinex => Bitfinex::new().get_price(symbol).await,
            CexExchange::Upbit => Upbit::new().get_price(symbol).await,
            CexExchange::Cryptocom => Cryptocom::new().get_price(symbol).await,
        }
    }

    /// Gets price from a DEX exchange
    async fn get_dex_price(
        exchange: &DexAggregator,
        base_token: &Token,
        quote_token: &Token,
        quote_amount: f64,
    ) -> Result<DexPrice, MarketScannerError> {
        match exchange {
            DexAggregator::KyberSwap => {
                KyberSwap::new()
                    .get_price(base_token, quote_token, quote_amount)
                    .await
            }
        }
    }

    /// Gets exchange name from Exchange enum
    fn exchange_name(exchange: &crate::common::Exchange) -> String {
        match exchange {
            crate::common::Exchange::Cex(cex) => match cex {
                CexExchange::Binance => "Binance",
                CexExchange::Bybit => "Bybit",
                CexExchange::MEXC => "MEXC",
                CexExchange::OKX => "OKX",
                CexExchange::Gateio => "Gateio",
                CexExchange::Kucoin => "Kucoin",
                CexExchange::Bitget => "Bitget",
                CexExchange::Btcturk => "Btcturk",
                CexExchange::Htx => "HTX",
                CexExchange::Coinbase => "Coinbase",
                CexExchange::Kraken => "Kraken",
                CexExchange::Bitfinex => "Bitfinex",
                CexExchange::Upbit => "Upbit",
                CexExchange::Cryptocom => "Crypto.com",
            }
            .to_string(),
            crate::common::Exchange::Dex(dex) => match dex {
                DexAggregator::KyberSwap => "KyberSwap",
            }
            .to_string(),
        }
    }
}
