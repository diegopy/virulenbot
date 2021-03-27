#![warn(
    clippy::all,
    clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    clippy::cargo
)]

use std::{
    convert::{Infallible, TryFrom},
    env,
    net::SocketAddr,
    sync::Arc,
};

use anyhow::{Context, Result};
use log::{debug, info};
use reqwest::StatusCode;
use serde::Serialize;
use serde_json::Value;
use teloxide::{
    payloads::SendMessage,
    prelude::*,
    requests::{HasPayload, JsonRequest, Payload},
    types::{Update, UpdateKind},
    utils::command::BotCommand,
};
use virulenbot::{
    cache::SymbolMapCacheStrategy,
    cg::CoinGeckoAPI,
    cmc::CoinMarketCapAPI,
    strategy::{MultiAPIQuoteStrategy, NamedPriceAPI},
    PriceQuotingStrategy, SymbolPriceQuote,
};
use warp::{
    reply::{self, Response},
    Filter, Rejection, Reply,
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

async fn process_command<T: PriceQuotingStrategy>(
    cx: UpdateWithCx<Bot, Message>,
    command: Command,
    strategy: &T,
) -> JsonRequest<SendMessage> {
    match command {
        Command::Help => cx.answer(Command::descriptions()),

        Command::Quote(ref symbol) => {
            let symbol = symbol.to_uppercase();
            let quoting_result = strategy.get_quote(&symbol, "usd").await;
            let reply_string = quoting_result
                .map(|r| format_quotes(&r))
                .unwrap_or_else(|error| format!("Error during price retrieval: {:#}", error));
            cx.answer(reply_string)
        }
    }
}

async fn webook_repl<T: PriceQuotingStrategy>(
    bot: Bot,
    bot_name: &str,
    update: Message,
    strategy: &T,
) -> Option<JsonRequest<SendMessage>> {
    if let Some(text) = update.text() {
        match Command::parse(text, bot_name) {
            Ok(command) => {
                let cx = UpdateWithCx {
                    requester: bot,
                    update,
                };
                Some(process_command(cx, command, strategy).await)
            }
            Err(error) => {
                debug!("Command parse error: {}", error);
                None
            }
        }
    } else {
        None
    }
}

async fn bot_repl<T: PriceQuotingStrategy>(
    cx: UpdateWithCx<Bot, Message>,
    command: Command,
    strategy: &T,
) -> Result<()> {
    let reply_request = process_command(cx, command, strategy).await;
    reply_request.send().await.context("Replying command")?;
    Ok(())
}

async fn handle_rejection(error: Rejection) -> Result<impl Reply, Infallible> {
    log::error!("Cannot process the request due to: {:?}", error);
    Ok(StatusCode::INTERNAL_SERVER_ERROR)
}

#[derive(Serialize)]
struct WebhookReply {
    method: String,

    #[serde(flatten)]
    payload: Value,
}

impl<T: Payload + Serialize> TryFrom<JsonRequest<T>> for WebhookReply {
    type Error = serde_json::Error;

    fn try_from(value: JsonRequest<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            method: T::NAME.to_owned(),
            payload: serde_json::to_value(value.payload_ref())?,
        })
    }
}

async fn webhook_handler<T: PriceQuotingStrategy>(
    json: Value,
    args: (Bot, String, Arc<T>),
) -> Result<Response, Rejection> {
    info!("In webhook handler with value {}", json);
    let (bot, bot_name, strategy) = args;
    let response = if let Ok(update) = Update::try_parse(&json) {
        match update.kind {
            UpdateKind::Message(message) => {
                webook_repl(bot.clone(), &bot_name, message, strategy.as_ref()).await
            }
            _ => None,
        }
    } else {
        None
    };
    let reply = response
        .map(|json_req| reply::json(&WebhookReply::try_from(json_req).unwrap()).into_response())
        .unwrap_or(StatusCode::OK.into_response());
    info!("webhook replying with {:?}", reply);
    Ok(reply)
}

fn with_data<T: PriceQuotingStrategy + Send + Sync>(
    bot: Bot,
    bot_name: String,
    strategy: Arc<T>,
) -> impl Filter<Extract = ((Bot, String, Arc<T>),), Error = std::convert::Infallible> + Clone {
    warp::any().map(move || (bot.clone(), bot_name.clone(), strategy.clone()))
}

async fn webhook_mode<T: PriceQuotingStrategy + Send + Sync + 'static>(
    bot: Bot,
    bot_name: &str,
    strategy: T,
) -> Result<()> {
    let endpoint = env::var("ENDPOINT").expect("ENDPOINT env variable");
    let port = env::var("PORT").expect("PORT env variable");
    bot.set_webhook(endpoint)
        .send()
        .await
        .expect("Cannot setup a webhook");
    let strategy = Arc::new(strategy);
    let server = warp::post()
        .and(warp::body::json())
        .and(with_data(bot, bot_name.to_owned(), strategy))
        .and_then(webhook_handler)
        .recover(handle_rejection);
    let server = warp::serve(server);
    tokio::spawn(
        server.run(
            ("0.0.0.0:".to_owned() + &port)
                .parse::<SocketAddr>()
                .unwrap(),
        ),
    )
    .await
    .map_err(|err| err.into())
}

async fn poll_mode<T: PriceQuotingStrategy + Send + Sync + 'static>(
    bot: Bot,
    bot_name: &'static str,
    strategy: T,
) -> Result<()> {
    bot.delete_webhook().send().await?;
    let container = Arc::new(strategy);
    teloxide::commands_repl(bot, bot_name, move |ctx, cmd| {
        let strategy = container.clone();
        async move { bot_repl(ctx, cmd, strategy.as_ref()).await }
    })
    .await;
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
        .unwrap_or("poll".to_owned());
    info!("Using mode {}", mode);
    if mode == "webhook" {
        webhook_mode(bot, "pepe", strategy).await
    } else {
        poll_mode(bot, "pepe", strategy).await
    }
}
