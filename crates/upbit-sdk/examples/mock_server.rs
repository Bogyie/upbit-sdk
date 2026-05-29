use upbit_sdk::{Credentials, SdkError, UpbitClient, UpbitConfig};

const MOCK_BASE_URL: &str = "http://127.0.0.1:8001/v1";

#[tokio::main]
async fn main() -> Result<(), SdkError> {
    let public_config = UpbitConfig::builder().base_url(MOCK_BASE_URL)?.build()?;
    let public_client = UpbitClient::new(public_config)?;
    let tickers = public_client
        .list_tickers(vec!["KRW-BTC".to_owned()])
        .await?;
    println!("mock returned {} ticker rows", tickers.len());

    let auth_config = UpbitConfig::builder()
        .base_url(MOCK_BASE_URL)?
        .credentials(Credentials::new("test-access-key", "test-secret-key")?)
        .build()?;
    let auth_client = UpbitClient::new(auth_config)?;
    let balances = auth_client.get_balance().await?;
    println!("mock returned {} balance rows", balances.len());

    Ok(())
}
