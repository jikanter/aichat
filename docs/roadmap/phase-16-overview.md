# Phase 16: Server Hardening : Overview - Epic 5

> **[DEFERRED 2026-04-17]** Phases 16, 17, and 18 are parked while Epic 9
> (Knowledge Evolution) is in flight. The existing `--serve` behavior is
> unchanged; expanding the server surface is a future-session decision.

| Item | Description | Status |
|---|---|---|
| 16A | Configurable CORS origins (`serve_cors_origins:` in config.yaml) | -- |
| 16B | Optional bearer token auth (`serve_api_key:`) | -- |
| 16C | Health endpoint (`GET /health`) | -- |
| 16D | Streaming usage in final SSE chunk | -- |
| 16E | Hot-reload endpoint (`POST /v1/reload`) | -- |
| 16F | Role metadata security (`RolePublicView` — hide prompt text, shell commands, filesystem paths) | -- |
| 16G | Single-role retrieval (`GET /v1/roles/{name}`) | -- |
| 16H | Cost in API responses (`usage.cost_usd` + `X-AIChat-Cost-USD` header) | -- |

## [Epic Details](./phase-16-server-hardening.md)
