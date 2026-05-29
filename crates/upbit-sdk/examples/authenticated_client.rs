use std::env;

use upbit_sdk::{Credentials, SdkError, UpbitClient, UpbitConfig};

#[tokio::main]
async fn main() -> Result<(), SdkError> {
    let access_key = env::var("UPBIT_ACCESS_KEY")
        .map_err(|_| SdkError::Config("UPBIT_ACCESS_KEY must be set by the caller".into()))?;
    let secret_key = env::var("UPBIT_SECRET_KEY")
        .map_err(|_| SdkError::Config("UPBIT_SECRET_KEY must be set by the caller".into()))?;

    let config = UpbitConfig::builder()
        .credentials(Credentials::new(access_key, secret_key)?)
        .build()?;
    let client = UpbitClient::new(config)?;

    match client.get_balance().await {
        Ok(balances) => {
            println!("received {} balance rows", balances.len());
            Ok(())
        }
        Err(SdkError::RateLimited { retry_after, .. }) => {
            eprintln!("rate limited; retry_after={retry_after:?}");
            Ok(())
        }
        Err(error) => Err(error),
    }
}
