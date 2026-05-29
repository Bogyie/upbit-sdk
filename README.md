# upbit-sdk

Rust workspace for an Upbit SDK and related test tooling.

## Workspace Layout

- `crates/upbit-sdk`: SDK crate foundation.
- `crates/upbit-mock`: spec-driven mock server and conformance baseline for
  SDK integration tests.
- `spec/upbit-rest-api.yaml`: machine-readable REST API contract seeded from
  official Upbit documentation research.
- `spec/README.md`: spec source, caveats, and regeneration policy.

The REST spec is the shared contract for SDK request/response types and mock
server route fixtures.

## Local Mock Server

Run a credential-free mock Upbit REST endpoint for SDK integration tests:

```sh
cargo run -p upbit-mock -- 127.0.0.1:8001 .
```

Use `http://127.0.0.1:8001/v1` as the SDK REST base URL. See
`crates/upbit-mock/README.md` for auth, validation, error, fixture, and
conformance details.

## SDK Endpoint Usage

Use the local mock for endpoint development and tests. Quotation mock examples
are credential-free:

```rust,no_run
use upbit_sdk::{CandleRequest, MinuteCandleUnit, UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .base_url("http://127.0.0.1:8001/v1")?
    .build()?;
let client = UpbitClient::new(config)?;

let candles = client
    .list_candles_minutes(
        MinuteCandleUnit::One,
        CandleRequest {
            market: "KRW-BTC".into(),
            count: Some(1),
            ..Default::default()
        },
    )
    .await?;
# Ok(())
# }
```

Auth-required exchange methods still need SDK credentials so the client can
build a bearer JWT, but local mock tests should use dummy, non-live values only:

```rust,no_run
use upbit_sdk::{Credentials, UpbitClient, UpbitConfig};

# fn example() -> Result<UpbitClient, upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .base_url("http://127.0.0.1:8001/v1")?
    .credentials(Credentials::new("test-access", "test-secret")?)
    .build()?;
UpbitClient::new(config)
# }
```

Live exchange endpoints require credentials and JWT signing. Keep live
credentials in the caller's secret store or environment, never in source code,
mock tests, or fixtures:

```rust,no_run
use upbit_sdk::{Credentials, UpbitClient, UpbitConfig};

# fn example(access_key: String, secret_key: String) -> Result<UpbitClient, upbit_sdk::SdkError> {
let config = UpbitConfig::builder()
    .credentials(Credentials::new(access_key, secret_key)?)
    .build()?;
UpbitClient::new(config)
# }
```

The SDK covers the 44 REST endpoints in `spec/upbit-rest-api.yaml`. The spec's
`list_subscriptions` inventory item is a WebSocket operation and is documented
outside the REST client surface.
