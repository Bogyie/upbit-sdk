# upbit-mock

`upbit-mock` provides a local, deterministic HTTP mock for SDK integration
tests. It reads `spec/upbit-rest-api.yaml` at startup and derives route
coverage, auth requirements, request-field validation, rate-limit headers, and
representative response fixtures from that contract.

## Run

From the repository root:

```sh
cargo run -p upbit-mock -- 127.0.0.1:8001 .
```

The first argument is the bind address. The second argument is the repository
root containing `spec/upbit-rest-api.yaml`. If omitted, the server binds to
`127.0.0.1:8001` and uses the current directory as the repository root.

Point SDK tests at `http://127.0.0.1:8001/v1` instead of
`https://api.upbit.com/v1`. Exchange endpoints require any bearer token-like
header, for example:

```sh
curl -H 'Authorization: Bearer test.jwt' http://127.0.0.1:8001/v1/accounts
```

Quotation endpoints are public:

```sh
curl 'http://127.0.0.1:8001/v1/ticker?markets=KRW-BTC'
```

## Fixture And Error Behavior

- Success fixtures are generated from each endpoint's `response_fields`.
- Required request fields are marked with `*` in the spec and return a 400
  Upbit-style error envelope when omitted.
- Path parameters declared in the spec are validated for presence in the route
  template and enum membership. For example, unsupported minute candle units
  return a 400 validation error instead of a success fixture.
- Auth-required endpoints return a 401 Upbit-style error envelope when the
  `Authorization: Bearer ...` header is missing.
- Add `__mock_error=rate_limit` to return a deterministic 429 rate-limit
  fixture.
- Responses include a `remaining-req` header derived from the endpoint
  `rate_limit_group`.

Current validation scope is limited to required request fields, declared path
parameter presence, path parameter enum values, bearer auth boundaries, and the
deterministic `__mock_error=rate_limit` branch. Other enum-like request/query
values are not validated unless they are modeled as `path_params` in the spec.

## Conformance Baseline

Run:

```sh
cargo test -p upbit-mock
```

The conformance tests fail when:

- a REST endpoint lacks a mock route,
- duplicate `METHOD path` route keys appear,
- a path template parameter is missing from `path_params`,
- a required `path_params` entry is missing from the route template,
- a path parameter enum accepts a value outside the spec contract,
- an endpoint fixture lacks a documented response field,
- auth-required endpoints stop returning a 401 without a bearer token, or
- the WebSocket-only `list_subscriptions` exception is no longer documented.

Current route coverage baseline:

- REST route coverage: 44/44
- Inventory exception: `list_subscriptions` (`protocol: websocket`)
