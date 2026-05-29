use upbit_sdk::{SdkError, UpbitClient, UpbitConfig};

#[tokio::main]
async fn main() -> Result<(), SdkError> {
    let client = UpbitClient::new(UpbitConfig::default())?;
    let tickers = client.list_tickers(vec!["KRW-BTC".to_owned()]).await?;

    for ticker in tickers {
        println!("{} last trade price: {}", ticker.market, ticker.trade_price);
    }

    Ok(())
}
