use anyhow::{Context, Result};
use futures::future;
use itertools::join;
use std::sync::Arc;
use teloxide::{prelude::*, utils::command::BotCommand};
use virulenbot::{
    cache::SymbolMapCacheStrategy, cg::CoinGeckoAPI, cmc::CoinMarketCapAPI, NamedAPI, PriceAPI,
};

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "VirulenBot commands:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "get token valuation in USD.")]
    Quote(String),
    //#[command(description = "clear the api caches.")]
    //ClearCache,
}

trait NamedPriceAPI: NamedAPI + PriceAPI + Sync {}
impl<T: NamedAPI + PriceAPI + Sync> NamedPriceAPI for T {}

async fn answer(cx: UpdateWithCx<Message>, command: Command, apis: &Apis) -> Result<()> {
    match command {
        Command::Help => cx
            .answer(Command::descriptions())
            .send()
            .await
            .context("Replying to Help")?,

        Command::Quote(ref symbol) => {
            let symbol = &symbol.to_uppercase();
            let price_getters: [&dyn NamedPriceAPI; 2] = [&apis.cmc, &apis.cg];
            let symbols = ["1"];
            let futures = price_getters.iter().map(|a| a.get_price(&symbols, "usd"));
            let results = future::join_all(futures).await;
            let reply_string = results
                .iter()
                .zip(price_getters.iter().map(|a| a.get_name()))
                .map(|(result, name)| match result {
                    Ok(prices) => format!(
                        "{}: {}",
                        name,
                        join(
                            prices
                                .iter()
                                .map(|(name, price)| format!("{} -> ${:.2}", name, price)),
                            ","
                        )
                    ),
                    Err(error) => format!("Error during price retrieval: {:#}", error),
                })
                .fold(format!("{} Price: \n", symbol), |mut acc, s| {
                    acc.push_str(&s);
                    acc.push('\n');
                    acc
                });
            cx.answer_str(reply_string)
                .await
                .context("Replying to Quote")?
        }
    };

    Ok(())
}

struct Apis {
    cmc: SymbolMapCacheStrategy<CoinMarketCapAPI>,
    cg: SymbolMapCacheStrategy<CoinGeckoAPI>,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().expect("Processing .env");
    env_logger::init();
    log::info!("Starting VirulenBot...");

    let bot = Bot::from_env();
    let token = std::env::var("COINMARKETCAP_TOKEN").context("Getting CoinMarketCap token")?;
    let apis = Apis {
        cmc: SymbolMapCacheStrategy::new(CoinMarketCapAPI::with_token(&token).unwrap()),
        cg: SymbolMapCacheStrategy::new(CoinGeckoAPI::build().unwrap()),
    };
    let container = Arc::new(apis);
    teloxide::commands_repl(bot, "VirulenBot", move |ctx, cmd| {
        let apis = container.clone();
        async move { answer(ctx, cmd, &apis).await }
    })
    .await;
    Ok(())
}
