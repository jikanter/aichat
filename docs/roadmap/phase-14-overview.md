# Phase 14: Capability Manifests : Overview - Epic 4

| Item | Description | Status |
|---|---|---|
| 14A | `capabilities:` field on roles (semantic intent tags) | -- |
| 14B | Human-readable port type annotations (derived from schema) | -- |
| 14C | Local capability resolver (`config.find_roles_by_capability("summarization")`) | -- |
| 14D | `--find-role` CLI flag (search by capability, input/output type) | -- |

**14A Design — Capabilities Field:**

```yaml
---
name: code-reviewer
description: Reviews code for bugs and security issues
capabilities: [code-review, security-audit, rust, python]
input_schema:
  type: object
  properties:
    code: { type: string }
    language: { type: string }
output_schema:
  type: object
  properties:
    issues: { type: array }
    severity: { type: string, enum: [low, medium, high, critical] }
---
```

Capabilities are free-form string tags. They enable discovery ("find me a role that can do code-review") without requiring formal ontology. This mirrors MCP Server Cards' approach to tool discovery.

**14C Design — Capability Resolver:**

```rust
// New method on Config
pub fn find_roles_by_capability(&self, capability: &str) -> Vec<&Role> {
    self.roles.iter()
        .filter(|r| r.capabilities().iter().any(|c| c.contains(capability)))
        .collect()
}

pub fn find_roles_by_port(&self, input_type: Option<&str>, output_type: Option<&str>) -> Vec<&Role> {
    self.roles.iter()
        .filter(|r| {
            let input_ok = input_type.map_or(true, |t| r.port_accepts(t));
            let output_ok = output_type.map_or(true, |t| r.port_produces(t));
            input_ok && output_ok
        })
        .collect()
}
```

**14D Design — Find Role CLI:**

```bash
$ aichat --find-role --capability code-review
  code-reviewer    in: {code, language}  out: {issues, severity}  capabilities: [code-review, security-audit]
  lint-checker     in: text              out: {errors}            capabilities: [code-review, linting]

$ aichat --find-role --accepts json --produces json
  classifier       in: json{text}        out: json{label, confidence}
  transformer      in: json{...}         out: json{...}
```

**Files:** `src/config/role.rs` (add `capabilities: Vec<String>`, `port_accepts()`, `port_produces()`), `src/config/mod.rs` (add resolver methods), `src/cli.rs` (add `--find-role` flag), `src/main.rs` (render results).
