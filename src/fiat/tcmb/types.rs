/// One currency from the TCMB bulletin, normalised to 1 unit (JPY is published per 100).
#[derive(Debug, Clone, PartialEq)]
pub struct TcmbCurrency {
    pub code: String,
    /// TRY paid by the bank for 1 unit (`ForexBuying` / `Unit`).
    pub forex_buying: f64,
    /// TRY asked by the bank for 1 unit (`ForexSelling` / `Unit`).
    pub forex_selling: f64,
}

/// A parsed `today.xml` bulletin.
#[derive(Debug, Clone)]
pub struct TcmbBulletin {
    /// Bulletin date, `YYYY-MM-DD`.
    pub date: String,
    pub currencies: Vec<TcmbCurrency>,
    /// Local time (ms since epoch) the bulletin was downloaded.
    pub fetched_at: u64,
}

impl TcmbBulletin {
    /// Buying/selling price of 1 unit of `code` in TRY. TRY itself is (1, 1).
    pub fn try_price(&self, code: &str) -> Option<(f64, f64)> {
        if code == "TRY" {
            return Some((1.0, 1.0));
        }
        self.currencies
            .iter()
            .find(|c| c.code == code)
            .map(|c| (c.forex_buying, c.forex_selling))
    }
}
