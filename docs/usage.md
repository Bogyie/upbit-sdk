# upbit-sdk Usage Guide

This guide shows safe SDK setup patterns for public quotation data,
authenticated Exchange API calls, local mock-server tests, retry/fallback
configuration, error handling, and logging redaction.

Do not commit real Upbit access keys, secret keys, JWTs, account identifiers,
order identifiers, or private trading data. Examples use placeholders and
read-only flows unless they target the local mock server.

## Public Quotation API

Quotation endpoints do not require credentials. The default client uses
`https://api.upbit.com/v1`.

```rust,no_run
use upbit_sdk::{UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let client = UpbitClient::new(UpbitConfig::default())?;
let tickers = client.list_tickers(vec!["KRW-BTC".to_owned()]).await?;

for ticker in tickers {
    println!("{} trade price: {}", ticker.market, ticker.trade_price);
}
# Ok(())
# }
```

For examples that should never leave the local machine, set the base URL to the
mock server instead.

## Authenticated Exchange API Setup

Authenticated endpoints need Upbit API credentials so the SDK can sign a bearer
JWT. Load credentials at runtime from environment variables or another caller
owned secret store.

```rust,no_run
use upbit_sdk::{Credentials, UpbitClient, UpbitConfig};

# fn example() -> Result<UpbitClient, Box<dyn std::error::Error>> {
let access_key = std::env::var("UPBIT_ACCESS_KEY")?;
let secret_key = std::env::var("UPBIT_SECRET_KEY")?;

let config = UpbitConfig::builder()
    .credentials(Credentials::new(access_key, secret_key)?)
    .build()?;

let client = UpbitClient::new(config)?;
# Ok(client)
# }
```

Use read-only account methods first, such as `get_balance` or `list_api_keys`.
Do not put live trading examples in docs or tests. Validate order-related code
against the mock server or a separately approved sandbox process.

## Local Mock Server

Start the mock server from the repository root:

```sh
cargo run -p upbit-mock -- 127.0.0.1:8001 .
```

Configure the SDK to use the loopback base URL:

```rust,no_run
use upbit_sdk::{UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .base_url("http://127.0.0.1:8001/v1")?
    .build()?;
let client = UpbitClient::new(config)?;

let tickers = client.list_tickers(vec!["KRW-BTC".to_owned()]).await?;
assert!(!tickers.is_empty());
# Ok(())
# }
```

For authenticated mock calls, provide dummy credentials. The SDK still signs a
request, while the mock server only checks that a bearer-token-like
authorization header is present.

```rust,no_run
use upbit_sdk::{Credentials, UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .base_url("http://127.0.0.1:8001/v1")?
    .credentials(Credentials::new("test-access-key", "test-secret-key")?)
    .build()?;
let client = UpbitClient::new(config)?;

let balances = client.get_balance().await?;
assert!(!balances.is_empty());
# Ok(())
# }
```

## Retry And Fallback

Retries and fallback routing are disabled by default. Enable them explicitly
for idempotent or otherwise safe flows.

```rust,no_run
use std::time::Duration;

use upbit_sdk::{UpbitClient, UpbitConfig};

# fn example() -> Result<UpbitClient, upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .retry_enabled(true)
    .retry_max_attempts(3)?
    .retry_backoff(Duration::from_millis(100), Duration::from_secs(2))?
    .fallback_enabled(true)
    .fallback_base_url("http://127.0.0.1:8001/v1")?
    .build()?;

let client = UpbitClient::new(config)?;
# Ok(client)
# }
```

Conservative defaults matter:

- retryable failures include transport errors, rate limits, and server errors;
- unsafe requests such as order mutation are not retried unless
  `retry_unsafe_requests(true)` is set;
- fallback requests are not sent for unsafe methods unless
  `fallback_allow_unsafe_requests(true)` is set;
- authenticated requests are not sent to fallback endpoints unless
  `fallback_allow_authenticated_requests(true)` is set.

Only enable unsafe or authenticated fallback when the alternate endpoint is
trusted and the operation has been reviewed for duplicate side effects.

## Error Handling

SDK methods return `Result<T, SdkError>`. Match typed variants when behavior
depends on the failure class.

```rust,no_run
use upbit_sdk::{SdkError, UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let client = UpbitClient::new(UpbitConfig::default())?;

match client.list_tickers(vec!["KRW-BTC".to_owned()]).await {
    Ok(tickers) => println!("received {} ticker rows", tickers.len()),
    Err(SdkError::RateLimited { retry_after, .. }) => {
        eprintln!("rate limited; retry_after={retry_after:?}");
    }
    Err(error) if error.is_retryable() => {
        eprintln!("retryable Upbit SDK error: {error}");
    }
    Err(error) => return Err(error),
}
# Ok(())
# }
```

Upbit error envelopes are exposed as typed `SdkError::Upbit` values when the
server returns a documented error body.

## Logging And Redaction

The client emits sanitized `tracing` fields for request attempts and retries.
It does not log request bodies. URL query values are redacted when their key
looks credential-, account-, order-, price-, or volume-related.

Use the public helpers before writing raw provider text to local diagnostics:

```rust
use upbit_sdk::redact_sensitive_text;

let raw = "Authorization: Bearer header.claims.signature access_key=raw";
let safe = redact_sensitive_text(raw);
assert!(!safe.contains("header.claims.signature"));
assert!(!safe.contains("raw"));
```

Never log authorization headers, JWTs, access keys, secret keys, raw order
payloads, account identifiers, or private balances.

## Crates.io Readiness

The crate metadata currently includes a description, repository, license,
readme, keywords, categories, Rust edition, Rust version, and an explicit
package include list. Packaging readiness should be checked without publishing:

```sh
cargo package -p upbit-sdk --allow-dirty --list
```

Known future publish blockers:

- final crate version policy and changelog/release notes must be approved;
- crates.io ownership/token setup must be handled outside this repository;
- the root README and crate README should be reviewed for public-facing wording;
- `cargo publish` must not be run until the release is explicitly authorized.
