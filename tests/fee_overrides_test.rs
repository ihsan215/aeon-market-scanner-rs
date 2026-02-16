use aeon_market_scanner_rs::common::CexPrice;
use aeon_market_scanner_rs::scanner::ArbitrageScanner;
use aeon_market_scanner_rs::{CexExchange, Exchange, FeeOverrides};

#[test]
fn fee_overrides_change_effective_prices_and_commission_percents() {
    // Deterministic/offline test: provide price snapshots directly.
    // Buy on Binance, sell on OKX.
    let buy = CexPrice {
        symbol: "BTCUSDT".to_string(),
        mid_price: 100.0,
        bid_price: 99.0,
        ask_price: 100.0,
        bid_qty: 1.0,
        ask_qty: 1.0,
        timestamp: 1,
        exchange: Exchange::Cex(CexExchange::Binance),
    };

    let sell = CexPrice {
        symbol: "BTCUSDT".to_string(),
        mid_price: 110.0,
        bid_price: 110.0,
        ask_price: 111.0,
        bid_qty: 1.0,
        ask_qty: 1.0,
        timestamp: 1,
        exchange: Exchange::Cex(CexExchange::OKX),
    };

    let base_opps =
        ArbitrageScanner::opportunities_from_prices(&[buy.clone(), sell.clone()], &[], None);
    let base = base_opps
        .iter()
        .find(|o| o.source_exchange == "Binance" && o.destination_exchange == "OKX")
        .expect("Expected a Binance -> OKX opportunity");

    // Defaults in commission table are 0.10% for both Binance and OKX.
    assert!((base.source_commission_percent - 0.1).abs() < 1e-9);
    assert!((base.destination_commission_percent - 0.1).abs() < 1e-9);
    // Default effective prices should reflect fee inclusion.
    assert!(base.effective_ask > buy.ask_price);
    assert!(base.effective_bid < sell.bid_price);

    // Now override fees: Binance 0.20%, OKX 0.05%.
    let overrides = FeeOverrides::default()
        .with_cex_taker_fee(CexExchange::Binance, 0.002)
        .with_cex_taker_fee(CexExchange::OKX, 0.0005);

    let ovr_opps = ArbitrageScanner::opportunities_from_prices(&[buy, sell], &[], Some(&overrides));
    let ovr = ovr_opps
        .iter()
        .find(|o| o.source_exchange == "Binance" && o.destination_exchange == "OKX")
        .expect("Expected a Binance -> OKX opportunity with overrides");

    // Commission percents should reflect overrides (decimal * 100).
    assert!((ovr.source_commission_percent - 0.2).abs() < 1e-9);
    assert!((ovr.destination_commission_percent - 0.05).abs() < 1e-9);

    // Effective ask should be higher with higher buy fee.
    assert!(ovr.effective_ask > base.effective_ask);
    // Effective bid should be higher with lower sell fee (less deducted).
    assert!(ovr.effective_bid > base.effective_bid);
}
