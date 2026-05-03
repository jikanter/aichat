# Phase 17: Role & Pipeline Execution : Overview - Epic 5

> **[DEFERRED 2026-04-17]** Phases 16, 17, and 18 are parked while Epic 9
> (Knowledge Evolution) is in flight.

| Item | Description | Status |
|---|---|---|
| 17A | Roles as virtual models (`model: "role:classify"` in `/v1/chat/completions`) | -- |
| 17B | Role invocation endpoint (`POST /v1/roles/{name}/invoke` — non-streaming) | -- |
| 17C | Role invocation endpoint (streaming with stage-boundary SSE events) | -- |
| 17D | Pipeline execution endpoint (`POST /v1/pipelines/run` — named or inline stages) | -- |
| 17E | Batch processing endpoint | -- |

**17A Design:** Roles appear as virtual models in `/v1/models`. OpenWebUI sees them in its model dropdown. Selecting `role:code-reviewer` transparently executes the full role pipeline. Zero changes to OpenWebUI.

**17B Design:** Dedicated endpoint with structured input, variables, model override, and trace:

```json
POST /v1/roles/classify/invoke
{
  "input": "Review this code for security issues...",
  "variables": {"language": "rust"},
  "model": "deepseek:deepseek-chat",
  "trace": true
}
```

Response includes `output`, `usage` (with `cost_usd`), `schema_valid`, and optional `trace` with per-stage breakdown.

## [Epic Details](./phase-17-server-execution.md)
