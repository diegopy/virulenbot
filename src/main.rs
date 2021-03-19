use std::sync::Arc;

use anyhow::{Context, Result};
use teloxide::{prelude::*, utils::command::BotCommand};
use virulenbot::{
    cache::SymbolMapCacheStrategy,
    cg::CoinGeckoAPI,
    cmc::CoinMarketCapAPI,
    strategy::{MultiAPIQuoteStrategy, NamedPriceAPI},
    PriceQuotingStrategy, SymbolPriceQuote,
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

fn format_quotes(quotes: &Vec<SymbolPriceQuote>) -> String {
    let mut result = String::new();
    for quote in quotes {
        result.push_str(&quote.source_name);
        result.push('\n');
        let matches_string = match quote.matches {
            Ok(ref token_prices) => {
                token_prices
                    .iter()
                    .fold("".to_owned(), |mut acc, token_price| {
                        acc.push_str(&format!(
                            "\t{} -> ${:.2}\n",
                            token_price.name, token_price.value
                        ));
                        acc
                    })
            }
            Err(ref error) => format!("Error during API price retrieval: {:#}", error),
        };
        result.push_str(&matches_string);
        result.push('\n');
    }
    result
}

async fn answer<T: PriceQuotingStrategy>(
    cx: UpdateWithCx<Message>,
    command: Command,
    strategy: &T,
) -> Result<()> {
    match command {
        Command::Help => cx
            .answer(Command::descriptions())
            .send()
            .await
            .context("Replying to Help")?,

        Command::Quote(ref symbol) => {
            let symbol = symbol.to_uppercase();
            let quoting_result = strategy.get_quote(&symbol, "usd").await;
            let reply_string = quoting_result
                .map(|r| format_quotes(&r))
                .unwrap_or_else(|error| format!("Error during price retrieval: {:#}", error));
            cx.answer_str(reply_string)
                .await
                .context("Replying to Quote")?
        }
    };

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().expect("Processing .env");
    env_logger::init();
    log::info!("Starting VirulenBot...");

    let bot = Bot::from_env();
    let token = std::env::var("COINMARKETCAP_TOKEN").context("Getting CoinMarketCap token")?;
    let apis: Vec<Box<dyn NamedPriceAPI>> = vec![
        Box::new(SymbolMapCacheStrategy::new(
            CoinMarketCapAPI::with_token(&token).unwrap(),
        )),
        Box::new(SymbolMapCacheStrategy::new(CoinGeckoAPI::build().unwrap())),
    ];
    let strategy = MultiAPIQuoteStrategy::new(apis);
    let container = Arc::new(strategy);
    teloxide::commands_repl(bot, "VirulenBot", move |ctx, cmd| {
        let strategy = container.clone();
        async move { answer::<MultiAPIQuoteStrategy>(ctx, cmd, &strategy).await }
    })
    .await;
    Ok(())
}
