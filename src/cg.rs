use std::collections::HashMap;

use crate::{NamedAPI, PriceAPI};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use log::info;
use reqwest::{header::HeaderMap, Client};
use serde::Deserialize;
use serde_json::Value;
pub struct CoinGeckoAPI {
    client: Client,
    //symbol_cache: Mutex<SymbolCache>,
    //max_cache_age: Duration,
}

#[derive(Deserialize)]
struct SymbolMap {
    id: String,
    symbol: String,
    //name: String,
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
        let result: Vec<SymbolMap> = res.json().await?;
        Ok(result)
    }

    /*     pub async fn get_id_for(&self, symbol: &str) -> Result<String> {
        let mut fresh_symbol_map = None::<Vec<SymbolMap>>;
        loop {
            {
                let mut cache = self.symbol_cache.lock();
                if let Some(symbols) = fresh_symbol_map {
                    cache.data.clear();
                    for symbol_data in symbols {
                        cache
                            .data
                            .insert(symbol_data.symbol.to_uppercase(), symbol_data.id);
                    }
                    cache.last_refresh = Instant::now();
                }
                if !cache.data.is_empty() && cache.last_refresh.elapsed() < self.max_cache_age {
                    return cache
                        .data
                        .get(symbol)
                        .ok_or(anyhow!("get_id_for: Missing id for symbol {}", symbol))
                        .map(|s| s.clone());
                }
            }
            fresh_symbol_map = Some(self.get_symbol_map().await?);
        }
    } */
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
        let res: Value = res.json().await?;
        info!("CoinGecko response {:?}", res);
        id_list
            .iter()
            .map(|id| {
                res[id][in_currency]
                    .as_f64()
                    .map(|price| (id.to_string(), price))
                    .ok_or(anyhow!("Cannot parse CoinGecko response"))
            })
            .collect()
    }

    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let map_data = self.get_map_data().await?;
        let mut result = HashMap::new();
        for datum in map_data {
            result
                .entry(datum.symbol.to_uppercase())
                .or_insert(vec![])
                .push(datum.id);
        }
        Ok(result)
        /*
        Ok(symbols
            .into_iter()
            .map(|sd| (sd.symbol.to_uppercase(), sd.id))
            .collect())*/
    }
}

impl NamedAPI for CoinGeckoAPI {
    fn get_name(&self) -> String {
        "CoinGecko".to_owned()
    }
}
