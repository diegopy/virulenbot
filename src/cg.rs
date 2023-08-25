use std::collections::HashMap;

use crate::{NamedAPI, PriceAPI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::debug;
use reqwest::{header::HeaderMap, Client};
use serde::Deserialize;
use serde_json::Value;
pub struct CoinGeckoAPI {
    client: Client,
}

#[derive(Deserialize)]
struct SymbolMap {
    id: String,
    symbol: String,
}

impl CoinGeckoAPI {
    pub fn build() -> Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("Accepts", "application/json".parse().unwrap());
        Ok(Self {
            client: Client::builder().default_headers(headers).build()?,
        })
    }

    async fn get_map_data(&self) -> Result<Vec<SymbolMap>> {
        let builder = self
            .client
            .get("https://api.coingecko.com/api/v3/coins/list");
        let res = builder
            .query(&[("include_platform", "false")])
            .send()
            .await?;
        res.json().await.map_err(|e| e.into())
    }
}
/*
[
  {
    "id": "01coin",
    "symbol": "zoc",
    "name": "01coin"
  },
  {
    "id": "0-5x-long-algorand-token",
    "symbol": "algohalf",
    "name": "0.5X Long Algorand Token"
  },
*/

#[async_trait]
impl PriceAPI for CoinGeckoAPI {
    async fn get_price(&self, id_list: &[&str], in_currency: &str) -> Result<Vec<(String, f64)>> {
        let builder = self
            .client
            .get("https://api.coingecko.com/api/v3/simple/price");
        let res = builder
            .query(&[
                ("ids", id_list.join(",").as_str()),
                ("vs_currencies", in_currency),
            ])
            .send()
            .await?;
        let status = res.status();
        let res: Value = res.json().await?;
        debug!("CoinGecko status: {}, response {:?}", status, res);
        id_list
            .iter()
            .map(|id| {
                res[id][in_currency]
                    .as_f64()
                    .map(|price| (id.to_string(), price))
                    .ok_or_else(|| anyhow!("Cannot parse CoinGecko response"))
            })
            .collect()
    }

    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let map_data = self.get_map_data().await?;
        let mut result = HashMap::new();
        for datum in map_data {
            result
                .entry(datum.symbol.to_uppercase())
                .or_insert_with(Vec::new)
                .push(datum.id);
        }
        Ok(result)
    }
}

impl NamedAPI for CoinGeckoAPI {
    fn get_name(&self) -> String {
        "CoinGecko".to_owned()
    }
}
