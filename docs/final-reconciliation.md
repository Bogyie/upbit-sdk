# Final Reconciliation Evidence

Status: implementation evidence for BOG-240  
Branch: `issue/BOG-240-final-spec-qa-security-pr-readiness`  
Base: `origin/integration/BOG-223-upbit-rust-sdk`  
Integration HEAD reviewed: `91d7ca62b9f1e5bbf6b7d8bd2694a9fda6f8a85b`  
Date: 2026-05-29

## Source Alignment

The repo-owned contract is `spec/upbit-rest-api.yaml`. It records BOG-228 as
the research source, `upbit-api-spec-draft.md` as the source attachment, and
official Upbit documentation verification at 2026-05-29 KST.

Endpoint inventory:

- total API inventory entries: 45
- REST endpoints: 44
- documented non-REST exception: `list_subscriptions` with `protocol:
  websocket`
- public REST endpoints: 13
- authenticated REST endpoints: 31

The BOG-228 and BOG-240 issue text refers to 45 REST endpoints, but
`spec/README.md` records the reconciliation decision: the current official REST
references enumerate 44 REST endpoints, while `list_subscriptions` is an
official non-REST WebSocket operation and is intentionally excluded from REST
SDK and HTTP mock route coverage.

## REST Coverage

SDK coverage is represented by
`crates/upbit-sdk/src/endpoints.rs::SUPPORTED_REST_ENDPOINT_IDS`. The endpoint
coverage test loads `spec/upbit-rest-api.yaml`, filters REST endpoints, and
asserts that the SDK's supported endpoint IDs exactly match the 44 REST
endpoint IDs from the spec.

Mock coverage is represented by `upbit_mock::MockServerSpec`, which derives
routes from the same spec. The mock coverage test asserts:

- total inventory endpoints: 45
- REST route count: 44
- documented exception: `list_subscriptions (websocket)`
- representative routes include `GET /market/all` and `POST /orders`

The mock conformance test also exercises every REST route generated from the
spec and verifies each route can produce a representative fixture response.

## Documentation And Packaging Evidence

BOG-242 is included in the reviewed integration branch through PR #6. The
following user-facing and package-readiness artifacts are present:

- `README.md`: install/setup, credential safety, mock-first flow,
  retry/fallback defaults, logging redaction, examples, and publishing notes
- `docs/usage.md`: public quotation API, authenticated API setup, mock server,
  retry/fallback, error handling, logging/redaction, and package-readiness
  examples
- `docs/publishing.md`: dry-run and real publish operating procedure, secret
  handling, release sequence, and failure/rollback notes
- `crates/upbit-sdk/examples/public_ticker.rs`
- `crates/upbit-sdk/examples/authenticated_client.rs`
- `crates/upbit-sdk/examples/mock_server.rs`
- `crates/upbit-sdk/Cargo.toml`: package metadata including description,
  repository, license, README, keywords, categories, and include list

No crates.io publish was performed as part of this reconciliation.

## Release Workflow Evidence

BOG-244 is included in the reviewed integration branch through PR #7, and
BOG-246 is included through PR #8. The reviewed integration HEAD is merge commit
`91d7ca62b9f1e5bbf6b7d8bd2694a9fda6f8a85b`.

`.github/workflows/publish-crates.yml` includes these safety controls:

- `pull_request`, integration-branch `push`, main-branch `push`, and
  `workflow_dispatch mode=dry-run` paths run the dry-run job only
- dry-run paths do not pass `registry-token` to the third-party action
- `katyo/publish-crates` is pinned to
  `02cc2f1ad653fb25c7d1ff9eb590a8a50d06186b`
- real publish can run on GitHub Release `published` events or separately
  authorized manual `workflow_dispatch mode=publish`
- real publish requires tag refs, `CARGO_REGISTRY_TOKEN`, and the protected
  `crates-io` environment
- release-triggered publishing validates `action == published`,
  `release.target_commitish`, and that the tag commit is reachable from
  `origin/main`
- checkout uses `fetch-depth: 0` for publish precondition validation

The workflow documentation in `docs/publishing.md` describes the main merge,
tag, GitHub Release, secret, environment, and failed-publish handling
requirements.

## Targeted Security And Code-Quality Review

Reviewed surfaces:

- JWT signing: `crates/upbit-sdk/src/auth.rs`
- credential and transport configuration: `crates/upbit-sdk/src/config.rs`
- request execution, retry, fallback, and logging: `crates/upbit-sdk/src/client.rs`
- error mapping: `crates/upbit-sdk/src/error.rs`
- redaction helpers: `crates/upbit-sdk/src/logging.rs`
- crates.io publish workflow: `.github/workflows/publish-crates.yml`

Security evidence:

- JWTs use HS512 and include a SHA512 `query_hash` when query parameters or
  JSON body data are signed.
- credential formatter output is redacted by wrapper types.
- authenticated requests validate authenticated transport before signing.
- retry and fallback are disabled by default.
- unsafe methods are not retried or sent to fallback endpoints unless explicitly
  enabled.
- authenticated requests are not sent to fallback endpoints unless explicitly
  enabled.
- SDK tracing uses redacted URLs and does not log request bodies.
- text redaction covers bearer tokens, access keys, secret keys, query hashes,
  UUID/order identifiers, price, and volume-like sensitive fields.
- publish dry-run paths avoid registry tokens, and real publish paths require
  GitHub secret/environment gates.

No remaining actionable security findings were identified from static review.
Residual operational risk remains around repository settings that cannot be
verified from code: `CARGO_REGISTRY_TOKEN`, `crates-io` environment protection,
protected `main` branch policy, and release approval process must be configured
in GitHub before any real publish.

## Local Verification

Commands run from the repository root:

| Command | Result |
| --- | --- |
| `ruby -ryaml -e '...'` endpoint inventory and SDK coverage checks | pass |
| `cargo fmt --check` | pass |
| `cargo clippy --workspace --all-targets -- -D warnings` | pass |
| `cargo test --workspace` | pass; `upbit-mock` 8 unit tests, `upbit-sdk` 37 unit tests, doc-tests 0/0 |
| `cargo test --workspace --doc` | pass; doc-tests 0/0 |
| `cargo package -p upbit-sdk --allow-dirty --list` | pass; listed package contents without publishing |

## Parent PR Readiness

Final PR to `main` is ready to be prepared after BOG-240 QA and required
review pass, subject to Repository Contribution Steward handling public PR
creation/update and repository contribution details. The final base-branch merge
and any crates.io publish or GitHub Release remain unauthorized without explicit
approval.
