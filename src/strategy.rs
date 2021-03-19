use crate::{NamedAPI, PriceAPI, PriceQuotingStrategy, SymbolPriceQuote, TokenQuote};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures::future;
use itertools::Itertools;

pub trait NamedPriceAPI: NamedAPI + PriceAPI + Send + Sync {}
impl<T: NamedAPI + PriceAPI + Send + Sync> NamedPriceAPI for T {}
pub struct MultiAPIQuoteStrategy {
    apis: Vec<Box<dyn NamedPriceAPI>>,
}

impl MultiAPIQuoteStrategy {
    pub fn new(apis: Vec<Box<dyn NamedPriceAPI>>) -> Self {
        Self { apis: apis }
    }
}

#[async_trait]
impl PriceQuotingStrategy for MultiAPIQuoteStrategy {
    async fn get_quote(
        &self,
        symbol: &str,
        in_currency: &str,
    ) -> Result<Vec<crate::SymbolPriceQuote>> {
        let quote_retriever = |i: usize| async move {
            let api = &self.apis[i];
            let symbol_map = api.get_symbol_map().await?;
            let maybe_ids = symbol_map
                .get(symbol)
                .map(|v| v.iter().map(|s| s.as_str()).collect_vec());
            if let Some(ids) = maybe_ids {
                api.get_price(&ids, in_currency).await
            } else {
                Err(anyhow!("{} not found in {} map", symbol, api.get_name()))
            }
        };
        let pending_quotes = (0..self.apis.len()).map(|i| quote_retriever(i));
        let quotes = future::join_all(pending_quotes).await;
        Ok(self
            .apis
            .iter()
            .zip(quotes.into_iter())
            .map(|(api, quote_result)| SymbolPriceQuote {
                source_name: api.get_name(),
                matches: quote_result.map(|quotes| {
                    quotes
                        .into_iter()
                        .map(|(name, value)| TokenQuote { name, value })
                        .collect()
                }),
            })
            .collect())
    }
}
