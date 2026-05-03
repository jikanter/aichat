# Phase 23: Role Evaluation : Overview - Epic 8

| Item | Description | Status |
|---|---|---|
| 23A | `metrics:` field on roles (shell commands that score output) | -- |
| 23B | `--compare` flag (run input through two roles, show results side-by-side with cost) | -- |
| 23C | Cost attribution by role in run log (tag each pipeline stage in JSONL) | -- |
| 23D | Role invocation history (append scored records to per-role ledger) | -- |

**23A Design — Metrics Field:**

```yaml
---
name: summarizer
metrics:
  - name: valid_json
    shell: "jq . >/dev/null 2>&1"
  - name: under_500_words
    shell: "test $(wc -w < /dev/stdin) -lt 500"
  - name: has_required_fields
    shell: "jq -e '.summary and .key_points' >/dev/null 2>&1"
---
```

Each metric receives the role's output on stdin and exits 0 (pass) or 1 (fail). Metrics run after output validation, before lifecycle hooks. Results recorded in the JSONL run log alongside cost and tokens.

**Implementation:** In `src/main.rs`, after `validate_schema("output", ...)`, iterate `role.metrics()`. For each, pipe output to the shell command. Record `{metric_name, pass: bool}` in the trace event.

**Files:** `src/config/role.rs` (add `metrics: Vec<RoleMetric>`), `src/main.rs` (evaluate metrics post-output), `src/utils/trace.rs` (emit metric events).

**23B Design — Compare Flag:**

```bash
$ echo "Review this code" | aichat --compare summarizer-v1 summarizer-v2

--- summarizer-v1 (deepseek:deepseek-chat) ---
  Output: { "summary": "...", "key_points": [...] }
  Metrics: valid_json=PASS  under_500_words=PASS  has_required_fields=PASS
  Cost: $0.0004  (892 input, 341 output tokens)

--- summarizer-v2 (claude:claude-haiku-4-5) ---
  Output: { "summary": "...", "key_points": [...] }
  Metrics: valid_json=PASS  under_500_words=PASS  has_required_fields=PASS
  Cost: $0.002  (892 input, 287 output tokens)

--- Comparison ---
  Cost ratio: summarizer-v2 is 5.0x more expensive
  Token ratio: summarizer-v2 uses 16% fewer output tokens
  Metrics: both pass all metrics
```

Manual A/B testing with zero infrastructure. Combined with the metrics field, this becomes systematic.

**Files:** `src/cli.rs` (add `--compare` flag taking two role names), `src/main.rs` (parallel execution and diff rendering).

**23C Design — Cost Attribution by Role:**

Currently the JSONL run log records the top-level role but not per-stage breakdown. Add `stage_role` and `pipeline_role` fields to each run log entry:

```jsonl
{"role":"extract","pipeline":"secure-review","stage":1,"model":"deepseek:deepseek-chat","cost_usd":0.0001,...}
{"role":"review","pipeline":"secure-review","stage":2,"model":"claude:claude-sonnet-4-6","cost_usd":0.012,...}
```

This enables downstream aggregation: `duckdb "SELECT role, SUM(cost_usd) FROM read_json('run.jsonl') GROUP BY role"`.

**Files:** `src/pipe.rs` (add `stage_role` + `pipeline_role` to trace/run log entries), `src/utils/ledger.rs` (extend run log schema).
