use virulenbot::{cmc::CoinMarketCapAPI, PriceAPI};

#[tokio::test]
async fn test_cmc() {
    dotenv::dotenv().expect("Processing .env");
    env_logger::init();
    let token = std::env::var("COINMARKETCAP_TOKEN").expect("Getting CoinMarketCap token");
    let api = CoinMarketCapAPI::with_token(&token).unwrap();
    assert!(api.get_price(&["BTC"], "usd").await.is_ok())
}
