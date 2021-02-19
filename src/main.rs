use anyhow::{Context, Result};
use teloxide::{prelude::*, utils::command::BotCommand};
use virulenbot::cmc::CoinMarketCapAPI;

#[derive(BotCommand)]
#[command(rename = "lowercase", description = "VirulenBot commands:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "get token valuation in USD.")]
    Quote(String),
}

async fn answer(cx: UpdateWithCx<Message>, command: Command, token: String) -> Result<()> {
    match command {
        Command::Help => cx
            .answer(Command::descriptions())
            .send()
            .await
            .context("Replying to Help")?,

        Command::Quote(ref symbol) => {
            let symbol = &symbol.to_uppercase();
            let api = CoinMarketCapAPI::with_token(&token).unwrap();
            let price = api.get_price(symbol).await?;
            cx.answer_str(format!("{} Price: ${:.2}", symbol, price))
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
    teloxide::commands_repl(bot, "VirulenBot", move |ctx, cmd| {
        let res = answer(ctx, cmd, token.clone());
        res
    })
    .await;
    Ok(())
}
