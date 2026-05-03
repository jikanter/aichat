# Phase 15: Contract Testing : Overview - Epic 4

| Item | Description | Status |
|---|---|---|
| 15A | Pipeline schema compatibility check at authoring time (`showboat validate-pipeline`) | -- |
| 15B | Cross-stage schema containment validation (output N satisfies input N+1) | -- |
| 15C | `--check` flag for validating role/pipeline definitions without execution | -- |

**15A Design — Authoring-Time Validation:**

```bash
$ showboat validate-pipeline extract-review-format

Pipeline: extract-review-format (3 stages)
  Stage 1: extract
    output_schema: { text: string, metadata: object }
  Stage 2: review
    input_schema:  { content: string, language: string }     # MISMATCH
    output_schema: { issues: array, severity: string }
  Stage 3: format
    input_schema:  { issues: array }                         # OK (subset)

FAIL: Stage 1 output -> Stage 2 input
  Missing: content, language
  Extra: text, metadata
  Suggestion: Add a transform role or update schemas for compatibility.
```

JSON Schema containment check: verify that a document conforming to output_schema would pass input_schema validation. This is deterministic — no LLM needed. Zero runtime cost, prevents an entire class of pipeline failures.

**Files:** `src/config/preflight.rs` (new: pipeline schema validation), integration with `showboat` command.
