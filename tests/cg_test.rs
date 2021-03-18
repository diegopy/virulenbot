use virulenbot::{cg::CoinGeckoAPI, PriceAPI};

#[tokio::test]
async fn test_cg() {
    let api = CoinGeckoAPI::build().unwrap();
    assert!(api
        .get_price(&["BTC"], "usd")
        .await
        .map_err(|e| {
            println!("{}", e);
            e
        })
        .is_ok())
}
