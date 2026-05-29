# Upbit REST API Spec

`upbit-rest-api.yaml` is the repo-owned contract for SDK and mock-server work.

## Source

- Research source: Multica issue BOG-228 attachment `upbit-api-spec-draft.md`
- Official docs checked: 2026-05-29 KST
- API overview: https://docs.upbit.com/kr/reference/api-overview
- Auth guide: https://docs.upbit.com/kr/reference/auth
- Rate limits: https://docs.upbit.com/kr/reference/rate-limits
- REST guide and errors: https://docs.upbit.com/kr/reference/rest-api-guide
- Discovery index: https://docs.upbit.com/kr/llms.txt

The research found no single official downloadable OpenAPI file. Official
reference pages expose per-endpoint OpenAPI JSON blocks plus prose constraints,
so this file preserves both structured fields and documented caveats.

## Regeneration Policy

Regenerate this file from official Upbit documentation before broad SDK type
generation or mock fixture generation. Keep `verified_at` and source URLs
current, and prefer official overview/auth/rate-limit guides over inconsistent
embedded OpenAPI metadata.

Do not hard-code runtime data such as live market lists, network states, fees,
withdrawal limits, supported orderbook levels, or wallet service status as SDK
constants. Treat those as API responses or refreshable test fixtures.

## Known Ambiguities

- Quotation REST endpoints are public even when embedded OpenAPI security says
  otherwise. Exchange REST endpoints require JWT auth even when embedded
  OpenAPI security is absent.
- Some response schemas have incomplete `required` lists. Generators must read
  `response_fields`, examples, and prose, not only official `required` arrays.
- Mutually exclusive request parameters are often documented in prose, including
  `uuid` vs `identifier`, `uuids[]` vs `identifiers[]`, and order-type-specific
  `price`/`volume` combinations.
- `time_in_force` prose appears to contain a typo. Use the order-type tables:
  `post_only` is limit-only, `best` requires `ioc` or `fok`, and `post_only`
  cannot be combined with `smp_type`.
- BOG-228 and this child issue refer to 45 REST endpoints, but the BOG-228
  REST table and current official REST references enumerate 44 REST endpoints.
  `list_subscriptions` is included as the 45th API inventory item with
  `protocol: websocket` because it is an official non-REST operation.
