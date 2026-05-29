# upbit-sdk crate

Rust SDK foundation for the Upbit REST API.

This crate exposes typed request and response models for the REST endpoint
inventory tracked in this repository, a reusable `reqwest`-backed client,
JWT signing for authenticated Exchange API calls, conservative retry and
fallback controls, and log redaction helpers.

Start with the repository README and usage guide:

- Repository README: <https://github.com/Bogyie/upbit-sdk>
- Usage guide: <https://github.com/Bogyie/upbit-sdk/blob/main/docs/usage.md>

Do not commit real Upbit access keys, secret keys, JWTs, account identifiers,
order identifiers, or private trading data. Use the local mock server for
development and examples whenever possible.
