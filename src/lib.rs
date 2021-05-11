pub mod cache;
pub mod cg;
pub mod cmc;
pub mod strategy;

use std::collections::HashMap;

use anyhow::{bail, Result};
use async_trait::async_trait;

#[async_trait]
pub trait PriceAPI {
    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>>;

    async fn get_price(&self, id_list: &[&str], in_currency: &str) -> Result<Vec<(String, f64)>>;
}

pub struct UnsupportedAPI {}

#[async_trait]
impl PriceAPI for UnsupportedAPI {
    async fn get_price(&self, _: &[&str], _: &str) -> Result<Vec<(String, f64)>> {
        bail!("Unsupported API")
    }

    async fn get_symbol_map(&self) -> Result<HashMap<String, Vec<String>>> {
        bail!("Unsupported API")
    }
}

pub trait NamedAPI {
    fn get_name(&self) -> String;
}

#[async_trait]
pub trait PriceQuotingStrategy {
    async fn get_quote(&self, symbol: &str, in_currency: &str) -> Result<Vec<SymbolPriceQuote>>;
}

pub struct SymbolPriceQuote {
    pub source_name: String,
    pub matches: Result<Vec<TokenQuote>>,
}

pub struct TokenQuote {
    pub name: String,
    pub value: f64,
}
