use crate::common::{
    AmountSide, CEXTrait, CexExchange, CexPrice, DEXTrait, DexAggregator, DexPrice, Exchange,
    FeeOverrides, MarketScannerError, effective_price_with_overrides, fee_rate_with_overrides,
};
use crate::dex::chains::Token;
use crate::{
    Binance, Bitfinex, Bitget, Btcturk, Bybit, Coinbase, Cryptocom, Gateio, Htx, Kraken, Kucoin,
    KyberSwap, Mexc, OKX, Upbit,
};
use futures::future::join_all;
use std::collections::HashMap;
use tokio::sync::mpsc;

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
        fee_overrides: Option<&FeeOverrides>,
    ) -> Result<Vec<ArbitrageOpportunity>, MarketScannerError> {
        // Fetch all prices in parallel
        let (cex_prices, dex_prices) = tokio::try_join!(
            Self::fetch_cex_prices(cex_exchanges, symbol),
            Self::fetch_dex_prices(dex_exchanges, base_token, quote_token, quote_amount)
        )?;

        // Find arbitrage opportunities by matching buy and sell candidates
        let opportunities =
            Self::opportunities_from_prices(&cex_prices, &dex_prices, fee_overrides);

        // Sort by profitability (most profitable first)
        let mut opportunities = opportunities;
        opportunities.sort_by(|a, b| {
            b.spread_percentage
                .partial_cmp(&a.spread_percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(opportunities)
    }

    /// Compute arbitrage opportunities from already-fetched price snapshots.
    ///
    /// This is useful if you want to provide your own price sources (or test deterministically)
    /// while still using the crate's matching/sorting logic. If `fee_overrides` is provided,
    /// all effective price and commission calculations will use it.
    pub fn opportunities_from_prices(
        cex_prices: &[CexPrice],
        dex_prices: &[DexPrice],
        fee_overrides: Option<&FeeOverrides>,
    ) -> Vec<ArbitrageOpportunity> {
        Self::find_opportunities(cex_prices, dex_prices, fee_overrides)
    }

    /// Connects to the given CEX WebSocket streams and continuously emits arbitrage
    /// opportunities as new prices arrive. Only exchanges that support WebSocket
    /// are used; others are skipped.
    ///
    /// Returns a receiver of opportunity snapshots (sorted by profitability).
    /// When all WS connections have closed, the receiver will receive `None`.
    pub async fn scan_arbitrage_from_websockets(
        symbols: &[&str],
        cex_exchanges: &[CexExchange],
        fee_overrides: Option<&FeeOverrides>,
        reconnect: bool,
        max_attempts: Option<u32>,
    ) -> Result<mpsc::Receiver<Vec<ArbitrageOpportunity>>, MarketScannerError> {
        let ws_exchanges: Vec<_> = cex_exchanges
            .iter()
            .filter(|ex| Self::exchange_supports_websocket(ex))
            .cloned()
            .collect();

        if ws_exchanges.is_empty() {
            return Err(MarketScannerError::ApiError(
                "No WebSocket-supported exchanges in the list".to_string(),
            ));
        }

        let mut receivers: Vec<(CexExchange, mpsc::Receiver<CexPrice>)> = Vec::new();
        for ex in &ws_exchanges {
            let rx =
                Self::stream_cex_prices_websocket(ex, symbols, reconnect, max_attempts).await?;
            receivers.push((ex.clone(), rx));
        }

        let (tx, rx) = mpsc::channel(64);
        let (tx_prices, mut rx_prices) = mpsc::channel::<CexPrice>(256);
        let symbols_vec: Vec<String> = symbols.iter().map(|s| (*s).to_string()).collect();
        let fee_overrides_owned = fee_overrides.cloned();

        for (_, mut ws_rx) in receivers {
            let tx_fwd = tx_prices.clone();
            tokio::spawn(async move {
                while let Some(price) = ws_rx.recv().await {
                    let _ = tx_fwd.send(price).await;
                }
            });
        }
        drop(tx_prices);

        tokio::spawn(async move {
            let mut cache: HashMap<(Exchange, String), CexPrice> = HashMap::new();
            let symbols_set: Vec<String> = symbols_vec;

            while let Some(price) = rx_prices.recv().await {
                // Geçersiz fiyatları atla; 0 gelen güncelleme önceki geçerli fiyatı üzerine yazmasın
                if price.mid_price <= 0.0 || price.bid_price <= 0.0 || price.ask_price <= 0.0 {
                    continue;
                }
                let symbol = price.symbol.clone();
                let ex = price.exchange.clone();
                cache.insert((ex.clone(), symbol.clone()), price);

                let mut all_opps = Vec::new();
                for symbol in &symbols_set {
                    let prices: Vec<CexPrice> = cache
                        .values()
                        .filter(|p| p.symbol == *symbol)
                        .cloned()
                        .collect();
                    if prices.len() >= 2 {
                        let opps = ArbitrageScanner::opportunities_from_prices(
                            &prices,
                            &[],
                            fee_overrides_owned.as_ref(),
                        );
                        all_opps.extend(opps);
                    }
                }
                all_opps.sort_by(|a, b| {
                    b.spread_percentage
                        .partial_cmp(&a.spread_percentage)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                if tx.send(all_opps).await.is_err() {
                    return;
                }
            }
        });

        Ok(rx)
    }

    fn exchange_supports_websocket(ex: &CexExchange) -> bool {
        match ex {
            CexExchange::Binance => Binance::new().supports_websocket(),
            CexExchange::Bybit => Bybit::new().supports_websocket(),
            CexExchange::MEXC => Mexc::new().supports_websocket(),
            CexExchange::OKX => OKX::new().supports_websocket(),
            CexExchange::Gateio => Gateio::new().supports_websocket(),
            CexExchange::Kucoin => Kucoin::new().supports_websocket(),
            CexExchange::Bitget => Bitget::new().supports_websocket(),
            CexExchange::Btcturk => Btcturk::new().supports_websocket(),
            CexExchange::Htx => Htx::new().supports_websocket(),
            CexExchange::Coinbase => Coinbase::new().supports_websocket(),
            CexExchange::Kraken => Kraken::new().supports_websocket(),
            CexExchange::Bitfinex => Bitfinex::new().supports_websocket(),
            CexExchange::Upbit => Upbit::new().supports_websocket(),
            CexExchange::Cryptocom => Cryptocom::new().supports_websocket(),
        }
    }

    async fn stream_cex_prices_websocket(
        exchange: &CexExchange,
        symbols: &[&str],
        reconnect: bool,
        max_attempts: Option<u32>,
    ) -> Result<mpsc::Receiver<CexPrice>, MarketScannerError> {
        match exchange {
            CexExchange::Binance => {
                Binance::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Bybit => {
                Bybit::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::MEXC => {
                Mexc::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::OKX => {
                OKX::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Gateio => {
                Gateio::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Kucoin => {
                Kucoin::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Bitget => {
                Bitget::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Btcturk => {
                Btcturk::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Htx => {
                Htx::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Coinbase => {
                Coinbase::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Kraken => {
                Kraken::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Bitfinex => {
                Bitfinex::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Upbit => {
                Upbit::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
            CexExchange::Cryptocom => {
                Cryptocom::new()
                    .stream_price_websocket(symbols, reconnect, max_attempts)
                    .await
            }
        }
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
            if let (Some(base), Some(quote), Some(amount)) = (base_token, quote_token, quote_amount)
            {
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
        fee_overrides: Option<&FeeOverrides>,
    ) -> Vec<ArbitrageOpportunity> {
        let mut opportunities = Vec::new();

        // Create buy candidates: effective ask = ask × (1 + fee), sorted lowest first
        let mut buy_candidates = Vec::new();
        for cex_price in cex_prices {
            let effective = effective_price_with_overrides(
                cex_price.ask_price,
                &cex_price.exchange,
                AmountSide::Buy,
                fee_overrides,
            );
            buy_candidates.push((
                effective,
                PriceData::Cex(cex_price.clone()),
                Self::exchange_name(&cex_price.exchange),
            ));
        }
        for dex_price in dex_prices {
            let effective = effective_price_with_overrides(
                dex_price.ask_price,
                &dex_price.exchange,
                AmountSide::Buy,
                fee_overrides,
            );
            buy_candidates.push((
                effective,
                PriceData::Dex(dex_price.clone()),
                Self::exchange_name(&dex_price.exchange),
            ));
        }
        buy_candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        // Create sell candidates: effective bid = bid × (1 − fee), sorted highest first
        let mut sell_candidates = Vec::new();
        for cex_price in cex_prices {
            let effective = effective_price_with_overrides(
                cex_price.bid_price,
                &cex_price.exchange,
                AmountSide::Sell,
                fee_overrides,
            );
            sell_candidates.push((
                effective,
                PriceData::Cex(cex_price.clone()),
                Self::exchange_name(&cex_price.exchange),
            ));
        }
        for dex_price in dex_prices {
            let effective = effective_price_with_overrides(
                dex_price.bid_price,
                &dex_price.exchange,
                AmountSide::Sell,
                fee_overrides,
            );
            sell_candidates.push((
                effective,
                PriceData::Dex(dex_price.clone()),
                Self::exchange_name(&dex_price.exchange),
            ));
        }
        sell_candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Match buy and sell candidates
        for (effective_ask, source_data, source_exchange) in &buy_candidates {
            for (effective_bid, dest_data, dest_exchange) in &sell_candidates {
                if source_exchange == dest_exchange || *effective_bid <= *effective_ask {
                    continue;
                }

                let spread = effective_bid - effective_ask;
                let spread_percentage = (spread / effective_ask) * 100.0;

                if spread_percentage < 0.01 {
                    continue;
                }

                let (symbol, buy_qty, sell_qty) = Self::extract_quantities(source_data, dest_data);
                let executable_quantity = buy_qty.min(sell_qty);

                let (src_comm_rate, dest_comm_rate) =
                    Self::extract_commission_rates(source_data, dest_data, fee_overrides);
                // Both in quote currency (e.g. USD): buy-side fee on notional, sell-side fee on notional
                let source_commission_quote =
                    *effective_ask * executable_quantity * (src_comm_rate / 100.0);
                let destination_commission_quote =
                    *effective_bid * executable_quantity * (dest_comm_rate / 100.0);
                let total_commission_quote = source_commission_quote + destination_commission_quote;

                opportunities.push(ArbitrageOpportunity {
                    source_exchange: source_exchange.clone(),
                    destination_exchange: dest_exchange.clone(),
                    symbol,
                    effective_ask: *effective_ask,
                    effective_bid: *effective_bid,
                    spread,
                    spread_percentage,
                    executable_quantity,
                    source_commission_percent: src_comm_rate,
                    destination_commission_percent: dest_comm_rate,
                    total_commission_quote,
                    source_leg: source_data.clone(),
                    destination_leg: dest_data.clone(),
                });
            }
        }

        opportunities
    }

    /// Extracts commission rates in percent from price data (e.g. 0.1 = 0.1%)
    fn extract_commission_rates(
        buy_data: &PriceData,
        sell_data: &PriceData,
        fee_overrides: Option<&FeeOverrides>,
    ) -> (f64, f64) {
        let src = match buy_data {
            PriceData::Cex(p) => fee_rate_with_overrides(&p.exchange, fee_overrides) * 100.0,
            PriceData::Dex(p) => fee_rate_with_overrides(&p.exchange, fee_overrides) * 100.0,
        };
        let dest = match sell_data {
            PriceData::Cex(p) => fee_rate_with_overrides(&p.exchange, fee_overrides) * 100.0,
            PriceData::Dex(p) => fee_rate_with_overrides(&p.exchange, fee_overrides) * 100.0,
        };
        (src, dest)
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
