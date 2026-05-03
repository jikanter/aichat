# Phase 19: RoleResolver & Unified Entity Resolution : Overview - Epic 6

| Item | Description | Status |
|---|---|---|
| 19A | `RoleResolver` trait (unified resolution across entity types) | -- |
| 19B | Unified entity resolution under `-r` (roles -> agents -> macros, with explicit `-a`/`--macro` overrides) | -- |
| 19C | Agent-in-pipeline (pipeline stages resolve agents via `to_role()` bridge) | -- |
| 19D | Agent MCP binding (`mcp_servers:` on AgentConfig, reuses Phase 6C machinery) | -- |

**19A Design — RoleResolver:**

```rust
pub trait RoleResolver {
    fn resolve(&self, address: &str) -> Result<ResolvedRole>;
    fn discover(&self, query: &CapabilityQuery) -> Result<Vec<RoleSummary>>;
}

pub enum RoleAddress {
    Local(String),                          // "review" -> roles/review.md
    Agent(String),                          // "agent:triage" -> agents/triage/
    Remote { host: String, role: String },  // "remote:staging:8080/review"
    Mcp { server: String, tool: String },   // "mcp:github/create_pr"
}

pub struct ResolvedRole {
    pub role: Role,
    pub source: RoleAddress,
    pub capabilities: Vec<String>,
}
```

**19B Design:** The `-r` flag uses unified resolution:

```rust
pub fn resolve_entity(&self, name: &str) -> Result<EntityRef> {
    // 1. Explicit prefix: "agent:foo", "remote:host/bar", "mcp:server/tool"
    if let Some(ref_) = self.resolve_prefixed(name)? { return Ok(ref_); }
    // 2. Local roles
    if let Ok(role) = self.retrieve_role(name) { return Ok(EntityRef::Role(role)); }
    // 3. Agents
    if self.agent_names().contains(&name.to_string()) { return Ok(EntityRef::Agent(name.to_string())); }
    // 4. Macros
    if self.macro_names().contains(&name.to_string()) { return Ok(EntityRef::Macro(name.to_string())); }
    bail!("Entity '{}' not found (checked roles, agents, macros)", name)
}
```

Backward compatible: `-a name` always resolves as agent. `--macro name` always resolves as macro.

**Files:** `src/config/resolver.rs` (new: RoleResolver trait + local impl), `src/config/mod.rs` (resolve_entity), `src/main.rs` (use resolve_entity for `-r`), `src/pipe.rs` (agent fallback in stage resolution), `src/config/agent.rs` (add `mcp_servers`).
