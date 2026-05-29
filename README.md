# upbit-sdk

Rust SDK workspace for the Upbit REST API and local mock testing tools.

The SDK is designed around a repo-owned REST contract in
`spec/upbit-rest-api.yaml`, typed request/response models, conservative client
defaults, and a credential-free mock server so development can happen without
live trading keys.

## Workspace Layout

- `crates/upbit-sdk`: SDK crate with REST endpoint methods, typed models,
  configuration, JWT signing, retry/fallback controls, and redaction helpers.
- `crates/upbit-mock`: spec-driven local mock server for SDK integration tests.
- `crates/upbit-sdk/examples`: runnable SDK examples for public, authenticated,
  and mock-server flows.
- `docs/usage.md`: detailed usage guide with safety and testing notes.
- `docs/publishing.md`: crates.io dry-run and controlled publish workflow notes.
- `spec/upbit-rest-api.yaml`: machine-readable REST API contract seeded from
  official Upbit documentation research.
- `spec/README.md`: spec source, caveats, and regeneration policy.

## Install

This repository is not published to crates.io yet. Use the git dependency form
or a local path while the crate is prepared for publishing:

```toml
[dependencies]
upbit-sdk = { git = "https://github.com/Bogyie/upbit-sdk", package = "upbit-sdk" }
```

For local workspace development:

```toml
[dependencies]
upbit-sdk = { path = "crates/upbit-sdk" }
```

Future crates.io publishing must be done separately and deliberately. Do not
run `cargo publish` without explicit release authorization and a reviewed
release checklist. The repository workflow `.github/workflows/publish-crates.yml`
uses a reviewed pinned equivalent of `katyo/publish-crates@v2`; pull request,
integration-branch, main-branch, and manual dry-run paths cannot publish
because they run with `dry-run: true` and do not pass a registry token to the
third-party action. Real publishing is triggered when a GitHub Release is
published for a tag that is already reachable from `main`, and it requires the
`CARGO_REGISTRY_TOKEN` GitHub secret plus the protected `crates-io`
environment. See `docs/publishing.md` for the full procedure.

## Feature Scope

The SDK covers the 44 REST endpoints tracked in `spec/upbit-rest-api.yaml`.
The spec inventory item `list_subscriptions` is a WebSocket operation and is
not part of the REST client surface.

Current SDK capabilities include:

- public Quotation API calls without credentials;
- authenticated Exchange API calls with JWT signing;
- typed endpoint request and response models;
- configurable base URL for live or loopback mock endpoints;
- optional bounded retries for retryable failures;
- optional fallback routing with unsafe/authenticated requests disabled by
  default;
- sanitized tracing fields and helper redaction utilities.

## Safety Defaults

`UpbitConfig::default()` is credential-free and conservative:

- automatic retry is disabled unless `retry_enabled(true)` or `RetryConfig` is
  supplied;
- fallback routing is disabled unless `fallback_enabled(true)` and an explicit
  fallback base URL are supplied;
- unsafe order-mutating requests are not retried or sent to fallback endpoints
  unless the caller explicitly allows that behavior;
- authenticated requests are not sent to fallback endpoints unless explicitly
  allowed;
- SDK trace fields redact URL query values that look like credentials, JWTs,
  account identifiers, order identifiers, price, or volume, and request bodies
  are not emitted by the client.

Keep live Upbit access keys, secret keys, JWTs, account identifiers, order
identifiers, and private trading data out of source code, fixtures, issue
comments, logs, and screenshots. Load credentials from the caller's secret
store or environment at runtime.

## Local Mock Server

Run a credential-free mock Upbit REST endpoint for SDK integration tests:

```sh
cargo run -p upbit-mock -- 127.0.0.1:8001 .
```

Use `http://127.0.0.1:8001/v1` as the SDK REST base URL. Quotation endpoints
are public. Exchange endpoints require any bearer-token-like header in the mock
server and dummy SDK credentials when using the SDK client. Never use live keys
for mock tests.

See `crates/upbit-mock/README.md` for auth, validation, error, fixture, and
conformance details.

## Basic Public Usage

```rust,no_run
use upbit_sdk::{UpbitClient, UpbitConfig};

# async fn example() -> Result<(), upbit_sdk::SdkError> {
let client = UpbitClient::new(UpbitConfig::default())?;
let tickers = client.list_tickers(vec!["KRW-BTC".to_owned()]).await?;

for ticker in tickers {
    println!("{} last trade price: {}", ticker.market, ticker.trade_price);
}
# Ok(())
# }
```

## Authenticated Setup

Authenticated Exchange API calls require credentials. Read them from the
environment or another secret store owned by the caller:

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

Avoid copy-pasteable live order examples. Prefer read-only account endpoints
and the mock server while validating integration code.

## Examples And Checks

Run examples against the live public API or a local mock as appropriate:

```sh
cargo run -p upbit-sdk --example public_ticker
cargo run -p upbit-sdk --example mock_server
UPBIT_ACCESS_KEY=placeholder UPBIT_SECRET_KEY=placeholder \
  cargo run -p upbit-sdk --example authenticated_client
```

Recommended local checks before opening a change:

```sh
cargo fmt --check
cargo test
cargo check -p upbit-sdk --examples
cargo package -p upbit-sdk --allow-dirty --list
```

`cargo package --list` is a non-publishing readiness check. It must not be
replaced with `cargo publish` unless a release is explicitly authorized.

## More Usage

See `docs/usage.md` for public quotation calls, authenticated client setup,
mock-server configuration, retry/fallback examples, error handling, logging and
redaction guidance, and current crates.io readiness notes.

See `docs/publishing.md` for crates.io dry-run checks, manual publish
preconditions, secret handling, and failed publish or rollback caveats.
