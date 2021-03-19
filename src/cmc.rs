use std::collections::HashMap;

use crate::{NamedAPI, PriceAPI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::info;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::Deserialize;
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
}

impl NamedAPI for CoinMarketCapAPI {
    fn get_name(&self) -> String {
        "CoinMarketCap".to_owned()
    }
}

#[async_trait]
impl PriceAPI for CoinMarketCapAPI {
    async fn get_price(&self, id_list: &[&str], in_currency: &str) -> Result<Vec<(String, f64)>> {
        info!("CMC called with {:?}", id_list);
        let builder = self
            .client
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest");
        let res = builder
            .query(&[("id", id_list.join(",").as_str()), ("convert", in_currency)])
            .send()
            .await?;
        let res: Value = res.json().await?;
        info!("CMC response {:?}", res);
        id_list
            .iter()
            .map(|id| {
                let entry = &res["data"][id];
                entry["quote"][in_currency.to_uppercase()]["price"]
                    .as_f64()
                    .and_then(|price| entry["name"].as_str().map(|name| (name.to_owned(), price)))
                    .ok_or(anyhow!("Can't parse CoinMarketCap response"))
            })
            .collect()
    }

    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let builder = self
            .client
            .get("https://pro-api.coinmarketcap.com/v1/cryptocurrency/map");
        let res = builder.send().await?;
        let map_data: IDMap = res.json().await?;
        let mut result = HashMap::new();
        for entry in map_data.data {
            result
                .entry(entry.symbol.to_uppercase())
                .or_insert(vec![])
                .push(entry.id.to_string());
        }
        Ok(result)
    }
}

#[derive(Deserialize)]
struct IDMap {
    data: Vec<IDMapEntry>,
}

#[derive(Deserialize)]
struct IDMapEntry {
    id: i32,
    symbol: String,
}
