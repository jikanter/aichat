# Phase 21: Pipeline DAG Primitives : Overview - Epic 7

| Item | Description | Status |
|---|---|---|
| 21A | Fan-out (run multiple stages in parallel on same input) | -- |
| 21B | Conditional routing (`when:` predicate routes to stage A or B) | -- |
| 21C | Merge (combine parallel outputs into single input for next stage) | -- |
| 21D | Pipeline DAG validation (cycle detection, unreachable nodes, type compatibility) | -- |

**21A/21B/21C Design — DAG Pipeline Syntax:**

Stays declarative YAML. Additive to existing sequential model — a `pipeline:` without `parallel:` or `when:` works exactly as today.

```yaml
# Sequential (unchanged)
pipeline:
  - role: extract
  - role: summarize

# Fan-out + merge
pipeline:
  - role: extract
  - parallel:
      - role: security-review
      - role: style-review
      - role: performance-review
    merge: concatenate          # concatenate | json_array | custom_role
  - role: synthesize

# Conditional routing
pipeline:
  - role: classify
  - switch:
      - when: { output_field: "category", equals: "bug" }
        role: bug-triage
      - when: { output_field: "category", equals: "feature" }
        role: feature-review
      - otherwise:
        role: general-review
  - role: format
```

**Implementation:**

`PipelineStage` becomes an enum:

```rust
enum PipelineNode {
    Stage(PipelineStage),                   // existing sequential stage
    Parallel {
        branches: Vec<PipelineNode>,
        merge: MergeStrategy,
    },
    Switch {
        conditions: Vec<ConditionalBranch>,
        otherwise: Option<Box<PipelineNode>>,
    },
}

enum MergeStrategy {
    Concatenate,                            // join outputs with newlines
    JsonArray,                              // wrap in JSON array
    CustomRole(String),                     // merge via a role
}

struct ConditionalBranch {
    when: Predicate,                        // JSONPath predicate on prior output
    node: PipelineNode,
}
```

**Fan-out runtime:** Uses existing `futures_util::future::join_all` from Phase 7D2. Each parallel branch gets a clone of the input. Branches execute concurrently.

**Conditional runtime:** Evaluate `when:` predicates against the previous stage's JSON output. Predicates are deterministic — `output_field`, `equals`, `contains`, `gt`, `lt` — no LLM call.

**Merge strategies:**
- `concatenate`: join outputs with `\n---\n` separator
- `json_array`: `[output1, output2, output3]`
- `custom_role`: pipe concatenated outputs through a merge role

**Files:** `src/pipe.rs` (refactor `PipelineStage` to `PipelineNode` enum, add parallel/conditional execution), `src/config/role.rs` (parse DAG YAML syntax).
