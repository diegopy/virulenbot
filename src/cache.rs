use crate::{NamedAPI, PriceAPI};
use anyhow::Result;
use async_trait::async_trait;
use parking_lot::Mutex;
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

pub struct SymbolMapCacheStrategy<T> {
    api: T,
    symbol_cache: Mutex<SymbolCache>,
    max_cache_age: Duration,
}

struct SymbolCache {
    data: HashMap<String, Vec<String>>,
    last_refresh: Instant,
}

impl<T> SymbolMapCacheStrategy<T> {
    pub fn new(api: T) -> Self {
        Self {
            api,
            symbol_cache: Mutex::new(SymbolCache {
                data: HashMap::new(),
                last_refresh: Instant::now(),
            }),
            max_cache_age: Duration::from_secs(3600 * 24),
        }
    }
}

#[async_trait]
impl<T: PriceAPI + Send + Sync> PriceAPI for SymbolMapCacheStrategy<T> {
    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>> {
        let mut fresh_symbol_map = None::<HashMap<String, Vec<String>>>;
        loop {
            {
                let mut cache = self.symbol_cache.lock();
                if let Some(symbols) = fresh_symbol_map {
                    cache.data = symbols;
                    cache.last_refresh = Instant::now();
                }
                if !cache.data.is_empty() && cache.last_refresh.elapsed() < self.max_cache_age {
                    return Ok(cache.data.clone());
                }
            }
            fresh_symbol_map = Some(self.get_symbol_map().await?);
        }
    }

    async fn get_price(&self, id_list: &[&str], in_currency: &str) -> Result<Vec<(String, f64)>> {
        self.api.get_price(id_list, in_currency).await
    }
}

impl<T: NamedAPI> NamedAPI for SymbolMapCacheStrategy<T> {
    fn get_name(&self) -> String {
        self.api.get_name()
    }
}
