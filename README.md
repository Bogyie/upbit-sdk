# upbit-sdk

Rust workspace for an Upbit SDK and related test tooling.

## Workspace Layout

- `crates/upbit-sdk`: SDK crate foundation.
- `crates/upbit-mock`: mock/test tooling crate foundation.
- `spec/upbit-rest-api.yaml`: machine-readable REST API contract seeded from
  official Upbit documentation research.
- `spec/README.md`: spec source, caveats, and regeneration policy.

The REST spec is the shared contract for SDK request/response types and mock
server route fixtures.

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

The client owns one reusable `reqwest::Client` per `UpbitClient` instance, so
retry and fallback attempts rebuild request objects while preserving the
underlying HTTP client's connection pooling.
