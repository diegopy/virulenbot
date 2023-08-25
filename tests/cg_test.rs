use virulenbot::{cg::CoinGeckoAPI, PriceAPI};

#[tokio::test]
async fn test_cg() {
    let _ = env_logger::builder().is_test(true).try_init();
    let api = CoinGeckoAPI::build().unwrap();
    assert!(api
        .get_price(&["bitcoin"], "usd")
        .await
        .map_err(|e| {
            eprintln!("{:?}", e);
            e
        })
        .is_ok())
}
