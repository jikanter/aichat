# Phase 11: Context Budget & Budget Propagation : Overview - Epic 2

*Merges existing Phase 11 with pipeline-level budget propagation.*

| Item | Description | Status |
|---|---|---|
| 11A | Context budget allocator core (`src/context_budget.rs`) | -- |
| 11B | BM25-ranked file inclusion (score files against query, fill budget by relevance) | -- |
| 11C | Budget-aware RAG (dynamic `top_k = remaining_budget / avg_chunk_tokens`) | -- |
| 11D | Pipeline budget propagation (`budget:` field, per-stage allocation) | -- |

**11D Design — Pipeline Budget Propagation:**

No framework currently propagates token budgets through a composition graph. A 4-stage pipeline doesn't know its total budget.

```yaml
pipeline:
  budget_usd: 0.05         # total pipeline budget
  stages:
    - role: extract          # gets proportional share
    - role: review
      budget_weight: 2.0     # gets 2x share
    - role: format
```

**Implementation:** In `pipe.rs:run()`, compute per-stage budgets from total. Pass budget to each `run_stage_inner()`. When a stage approaches budget, signal truncation rather than failure. Reuses Phase 11A's `ContextBudget` allocator per stage.

This turns "cost-conscious" from a cultural norm into an architectural guarantee.

**Files:** `src/pipe.rs` (budget allocation + enforcement), `src/config/role.rs` (pipeline `budget_usd:`, stage `budget_weight:`).

## [Epic Details](./phase-11-context-budget.md)
