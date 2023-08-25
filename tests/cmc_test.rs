use virulenbot::{cmc::CoinMarketCapAPI, PriceAPI};

#[tokio::test]
#[ignore]
async fn test_cmc() {
    dotenv::dotenv().expect("Processing .env");
    let _ = env_logger::builder().is_test(true).try_init();
    let token = std::env::var("COINMARKETCAP_TOKEN").expect("Getting CoinMarketCap token");
    let api = CoinMarketCapAPI::with_token(&token).unwrap();
    assert!(api.get_price(&["BTC"], "usd").await.is_ok())
}
