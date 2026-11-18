mod types;

use async_trait::async_trait;

use crate::common::{MarketScannerError, create_http_client, find_mid_price, get_timestamp_millis};
use crate::fiat::tcmb::types::{TcmbBulletin, TcmbCurrency};
use crate::fiat::{FiatProvider, FiatRate, FiatTrait, parse_fiat_pair};

const TCMB_TODAY_URL: &str = "https://www.tcmb.gov.tr/kurlar/today.xml";

/// Indicative exchange rates of the Central Bank of the Republic of Türkiye (TCMB).
/// No API key needed.
///
/// - About 20 currencies against TRY (USD, EUR, GBP, CHF, JPY, CNY, AED, ...), published once
///   per working day around 15:30 Istanbul time. Outside that, `today.xml` serves the latest
///   bulletin, and [`FiatRate::date`] tells you which day it is from.
/// - Unlike mid-only sources it publishes buying and selling prices, which this provider
///   returns as [`FiatRate::bid`] / [`FiatRate::ask`] ("forex" prices, not banknote prices).
/// - Any pair of listed currencies works: `USD/TRY`, `TRY/EUR`, and crosses such as
///   `EUR/USD`, computed through TRY.
/// - Every call downloads the bulletin; it changes once per working day, so cache the results
///   on your side. [`FiatRate::timestamp`] is when it was downloaded.
pub struct Tcmb {
    client: reqwest::Client,
}

impl Default for Tcmb {
    fn default() -> Self {
        Self::new()
    }
}

impl Tcmb {
    pub fn new() -> Self {
        Self {
            client: create_http_client(),
        }
    }

    /// Every currency in the latest bulletin quoted against TRY (`USD/TRY`, `EUR/TRY`, ...).
    pub async fn get_all_rates(&self) -> Result<Vec<FiatRate>, MarketScannerError> {
        let bulletin = self.fetch_bulletin().await?;
        Ok(bulletin
            .currencies
            .iter()
            .filter_map(|c| to_rate(&bulletin, &c.code, "TRY"))
            .collect())
    }

    async fn fetch_bulletin(&self) -> Result<TcmbBulletin, MarketScannerError> {
        let response = self.client.get(TCMB_TODAY_URL).send().await?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(MarketScannerError::ApiError(format!(
                "TCMB error: {status} - {}",
                body.chars().take(200).collect::<String>()
            )));
        }
        parse_bulletin(&response.text().await?)
    }
}

#[async_trait]
impl FiatTrait for Tcmb {
    fn provider(&self) -> FiatProvider {
        FiatProvider::Tcmb
    }

    fn provider_name(&self) -> &str {
        "TCMB"
    }

    async fn health_check(&self) -> Result<(), MarketScannerError> {
        self.get_rate("USD/TRY").await.map(|_| ())
    }

    async fn get_rate(&self, pair: &str) -> Result<FiatRate, MarketScannerError> {
        let (base, quote) = parse_fiat_pair(pair)?;
        let bulletin = self.fetch_bulletin().await?;
        to_rate(&bulletin, &base, &quote).ok_or_else(|| unsupported(&base, &quote))
    }

    /// All pairs come from a single bulletin download; results keep input order.
    async fn get_rates(&self, pairs: &[&str]) -> Result<Vec<FiatRate>, MarketScannerError> {
        let parsed = pairs
            .iter()
            .map(|pair| parse_fiat_pair(pair))
            .collect::<Result<Vec<_>, _>>()?;
        let bulletin = self.fetch_bulletin().await?;
        parsed
            .iter()
            .map(|(base, quote)| {
                to_rate(&bulletin, base, quote).ok_or_else(|| unsupported(base, quote))
            })
            .collect()
    }
}

fn unsupported(base: &str, quote: &str) -> MarketScannerError {
    MarketScannerError::InvalidSymbol(format!("TCMB has no rate for {base}/{quote}"))
}

/// `base/quote` through TRY. Bid is what you get selling 1 base for quote, ask what you pay:
/// bid = base_buying / quote_selling, ask = base_selling / quote_buying.
fn to_rate(bulletin: &TcmbBulletin, base: &str, quote: &str) -> Option<FiatRate> {
    let (base_buy, base_sell) = bulletin.try_price(base)?;
    let (quote_buy, quote_sell) = bulletin.try_price(quote)?;
    let bid = base_buy / quote_sell;
    let ask = base_sell / quote_buy;
    if !(bid.is_finite() && ask.is_finite() && bid > 0.0 && ask > 0.0) {
        return None;
    }
    Some(FiatRate {
        pair: format!("{base}/{quote}"),
        base: base.to_string(),
        quote: quote.to_string(),
        rate: find_mid_price(bid, ask),
        bid: Some(bid),
        ask: Some(ask),
        date: bulletin.date.clone(),
        timestamp: bulletin.fetched_at,
        exchange_timestamp: None,
        provider: FiatProvider::Tcmb,
    })
}

/// Parses `today.xml`. Currencies without both forex prices (e.g. XDR) are skipped.
fn parse_bulletin(xml: &str) -> Result<TcmbBulletin, MarketScannerError> {
    let parse_err =
        |what: &str| MarketScannerError::ApiError(format!("TCMB: could not parse {what}"));

    let header = xml
        .find("<Tarih_Date")
        .map(|i| &xml[i..])
        .ok_or_else(|| parse_err("bulletin header"))?;
    let raw_date = attribute(header, "Tarih").ok_or_else(|| parse_err("bulletin date"))?;
    let date = chrono::NaiveDate::parse_from_str(raw_date, "%d.%m.%Y")
        .map_err(|_| parse_err("bulletin date"))?
        .format("%Y-%m-%d")
        .to_string();

    let mut currencies = Vec::new();
    for block in xml.split("<Currency ").skip(1) {
        let block = block.split("</Currency>").next().unwrap_or(block);
        let Some(code) = attribute(block, "CurrencyCode") else {
            continue;
        };
        let number = |tag: &str| element(block, tag).and_then(|v| v.trim().parse::<f64>().ok());
        let unit = number("Unit").filter(|u| *u > 0.0).unwrap_or(1.0);
        if let (Some(buying), Some(selling)) = (number("ForexBuying"), number("ForexSelling")) {
            if buying > 0.0 && selling > 0.0 {
                currencies.push(TcmbCurrency {
                    code: code.to_uppercase(),
                    forex_buying: buying / unit,
                    forex_selling: selling / unit,
                });
            }
        }
    }

    if currencies.is_empty() {
        return Err(parse_err("any currency"));
    }
    Ok(TcmbBulletin {
        date,
        currencies,
        fetched_at: get_timestamp_millis(),
    })
}

/// Value of `name="..."` in the first tag of `s`.
fn attribute<'a>(s: &'a str, name: &str) -> Option<&'a str> {
    let tag_end = s.find('>')?;
    let tag = &s[..tag_end];
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// Text of `<tag>...</tag>`; `None` for missing or self-closing (`<tag/>`) elements.
fn element<'a>(s: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = s.find(&open)? + open.len();
    let len = s[start..].find(&close)?;
    Some(&s[start..start + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<?xml-stylesheet type="text/xsl" href="isokur.xsl"?>
<Tarih_Date Tarih="17.11.2026" Date="11/17/2026"  Bulten_No="2026/216" >
	<Currency CrossOrder="0" Kod="USD" CurrencyCode="USD">
			<Unit>1</Unit>
			<Isim>ABD DOLARI</Isim>
			<ForexBuying>50.3292</ForexBuying>
			<ForexSelling>50.4199</ForexSelling>
			<CrossRateUSD/>
	</Currency>
	<Currency CrossOrder="9" Kod="EUR" CurrencyCode="EUR">
			<Unit>1</Unit>
			<ForexBuying>55.3449</ForexBuying>
			<ForexSelling>55.4446</ForexSelling>
	</Currency>
	<Currency CrossOrder="11" Kod="JPY" CurrencyCode="JPY">
			<Unit>100</Unit>
			<ForexBuying>31.5415</ForexBuying>
			<ForexSelling>31.7504</ForexSelling>
	</Currency>
	<Currency CrossOrder="20" Kod="XDR" CurrencyCode="XDR">
			<Unit>1</Unit>
			<ForexBuying>67.7673</ForexBuying>
			<ForexSelling></ForexSelling>
	</Currency>
</Tarih_Date>"#;

    fn approx(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[test]
    fn parses_bulletin() {
        let bulletin = parse_bulletin(SAMPLE).unwrap();
        assert_eq!(bulletin.date, "2026-11-17");
        let codes: Vec<&str> = bulletin
            .currencies
            .iter()
            .map(|c| c.code.as_str())
            .collect();
        assert_eq!(codes, ["USD", "EUR", "JPY"], "XDR has no selling price");
        let jpy = &bulletin.currencies[2];
        assert!(approx(jpy.forex_buying, 0.315415), "JPY is per 100 units");
    }

    #[test]
    fn computes_direct_inverse_and_cross_rates() {
        let bulletin = parse_bulletin(SAMPLE).unwrap();

        let usd_try = to_rate(&bulletin, "USD", "TRY").unwrap();
        assert!(approx(usd_try.bid.unwrap(), 50.3292));
        assert!(approx(usd_try.ask.unwrap(), 50.4199));
        assert!(approx(usd_try.rate, (50.3292 + 50.4199) / 2.0));
        assert_eq!(usd_try.provider, FiatProvider::Tcmb);

        let try_usd = to_rate(&bulletin, "TRY", "USD").unwrap();
        assert!(approx(try_usd.bid.unwrap(), 1.0 / 50.4199));
        assert!(approx(try_usd.ask.unwrap(), 1.0 / 50.3292));

        let eur_usd = to_rate(&bulletin, "EUR", "USD").unwrap();
        assert!(approx(eur_usd.bid.unwrap(), 55.3449 / 50.4199));
        assert!(approx(eur_usd.ask.unwrap(), 55.4446 / 50.3292));
        assert!(eur_usd.bid.unwrap() < eur_usd.ask.unwrap());

        assert!(to_rate(&bulletin, "XDR", "TRY").is_none());
        assert!(to_rate(&bulletin, "GBP", "TRY").is_none());
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_bulletin("<html>maintenance</html>").is_err());
    }
}
