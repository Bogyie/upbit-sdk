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
