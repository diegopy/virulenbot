use std::{env, sync::Arc};

use anyhow::{Context, Result};
use log::info;
use teloxide::{prelude::*, update_listeners::webhooks, utils::command::BotCommands};
use virulenbot::{
    cache::SymbolMapCacheStrategy,
    cg::CoinGeckoAPI,
    cmc::CoinMarketCapAPI,
    strategy::{MultiAPIQuoteStrategy, NamedPriceAPI},
    PriceQuotingStrategy, SymbolPriceQuote,
};

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "VirulenBot v2.0 commands:")]
enum VirulenCommand {
    #[command(description = "Display this text.")]
    Help,
    #[command(description = "Get token valuation in USD.")]
    Quote(String),
    //#[command(description = "clear the api caches.")]
    //ClearCache,
}

fn format_quotes(quotes: &[SymbolPriceQuote]) -> String {
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

async fn process_command<T: PriceQuotingStrategy>(
    bot: Bot,
    message: Message,
    command: VirulenCommand,
    strategy: &T,
) -> ResponseResult<()> {
    match command {
        VirulenCommand::Help => {
            bot.send_message(message.chat.id, VirulenCommand::descriptions().to_string())
                .await?
        }

        VirulenCommand::Quote(ref symbol) => {
            let symbol = symbol.to_uppercase();
            let quoting_result = strategy.get_quote(&symbol, "usd").await;
            let reply_string = quoting_result
                .map(|r| format_quotes(&r))
                .unwrap_or_else(|error| format!("Error during price retrieval: {:#}", error));
            bot.send_message(message.chat.id, reply_string).await?
        }
    };
    Ok(())
}

async fn webhook_mode<T: PriceQuotingStrategy + Send + Sync + 'static>(
    bot: Bot,
    strategy: T,
) -> Result<()> {
    let endpoint = env::var("ENDPOINT")
        .expect("ENDPOINT env variable")
        .as_str()
        .try_into()
        .expect("invalid endpoint");
    let port = env::var("PORT")
        .expect("PORT env variable")
        .parse()
        .expect("invalid port");
    let container = Arc::new(strategy);
    let addr = ([0, 0, 0, 0], port).into();
    let listener = webhooks::axum(bot.clone(), webhooks::Options::new(addr, endpoint)).await?;
    let handler = move |bot, msg, cmd| {
        let strat = container.clone();
        async move {
            process_command(bot, msg, cmd, strat.as_ref()).await?;
            Ok(())
        }
    };
    VirulenCommand::repl_with_listener(bot, handler, listener).await;
    Ok(())
}

async fn poll_mode<T: PriceQuotingStrategy + Send + Sync + 'static>(
    bot: Bot,
    strategy: T,
) -> Result<()> {
    bot.delete_webhook().send().await?;
    let container = Arc::new(strategy);
    let handler = move |bot, msg, cmd| {
        let strat = container.clone();
        async move {
            process_command(bot, msg, cmd, strat.as_ref()).await?;
            Ok(())
        }
    };
    VirulenCommand::repl(bot, handler).await;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv::dotenv().ok();
    env_logger::init();
    info!("Starting VirulenBot...");

    let bot = Bot::from_env();
    let token = std::env::var("COINMARKETCAP_TOKEN").context("Getting CoinMarketCap token")?;
    let apis: Vec<Box<dyn NamedPriceAPI>> = vec![
        Box::new(SymbolMapCacheStrategy::new(
            CoinMarketCapAPI::with_token(&token).unwrap(),
        )),
        Box::new(SymbolMapCacheStrategy::new(CoinGeckoAPI::build().unwrap())),
    ];
    let strategy = MultiAPIQuoteStrategy::new(apis);
    let mode = std::env::var("MODE")
        .context("Getting MODE env var")
        .unwrap_or_else(|_| "poll".to_owned());
    info!("Using mode {}", mode);
    if mode == "webhook" {
        webhook_mode(bot, strategy).await
    } else {
        poll_mode(bot, strategy).await
    }
}
