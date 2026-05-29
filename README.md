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
