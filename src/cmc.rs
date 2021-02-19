use anyhow::{Error, Result};
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde_json::Value;
pub struct CoinMarketCapAPI {
    client: Client,
}

impl CoinMarketCapAPI {
    pub fn with_token(token: &str) -> Result<Self> {
        let mut token_value = HeaderValue::from_str(token)?;
        token_value.set_sensitive(true);
        let mut headers = HeaderMap::new();
        headers.insert("X-CMC_PRO_API_KEY", token_value);
        headers.insert("Accepts", "application/json".parse().unwrap());
        Ok(Self {
            client: Client::builder().default_headers(headers).build()?,
        })
    }
    pub async fn get_price(&self, symbol: &str) -> Result<f64> {
        let builder = self
            .client
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest");
        let res = builder.query(&[("symbol", symbol)]).send().await?;
        let res: Value = res.json().await?;
        res["data"][symbol]["quote"]["USD"]["price"]
            .as_f64()
            .ok_or(Error::msg("Cannot parse CoinMarketCap response"))
    }
}
