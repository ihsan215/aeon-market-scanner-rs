/// Bitfinex v2 API orderbook response format
/// Returns array of arrays: [[price, count, amount], ...]
/// where amount is negative for bids and positive for asks
/// [price, count, amount] formatında
pub type BitfinexOrderBookResponse = Vec<[f64; 3]>;

/// Bitfinex platform status response
/// Returns [1] for operational, [0] for maintenance
pub type BitfinexPlatformStatus = Vec<i64>;
