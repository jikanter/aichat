# Phase 12: Discoverability & Previews : Overview - Epic 3

| Item | Description | Status |
|---|---|---|
| 12A | Resolved prompt preview (`--dry-run` with `extends`/`include` expanded, variables interpolated) | -- |
| 12B | Pipeline visualization in `--dry-run` (text diagram: `extract -> validate -> summarize (3 stages)`) | -- |
| 12C | Port signatures in `--list-roles` (`--verbose` shows `in: raw-text, out: json{summary, entities}`) | -- |
| 12D | Composition summary after `.role <name>` in REPL (`extends: base, includes: [safety], tools: 3`) | -- |

**12A/12B Design — Resolved Preview:**

`--dry-run` already exists but shows the raw prompt. Enhance it to render the *fully resolved* state:

```bash
$ aichat -r code-reviewer --dry-run "review this"

--- Resolved Role: code-reviewer ---
  extends: base-analyst
  includes: [json-output, safety-checks]
  model: claude:claude-sonnet-4-6
  tools: 3 (web_search, fs_cat, execute_command)
  input_schema: { type: "string" }
  output_schema: { properties: { issues: [...], severity: [...] } }

--- Pipeline ---
  1. extract (deepseek:deepseek-chat)
  2. review (claude:claude-sonnet-4-6)
  3. format (deepseek:deepseek-chat)

--- Assembled Prompt (847 tokens) ---
  [system] You are a code review assistant...
  [user] review this

--- Estimated Cost ---
  $0.003 (3 stages, ~2400 tokens total)
```

Zero tokens spent. This is the "terraform plan" moment — the most beloved command in that ecosystem because it eliminates the fear of "what will this actually do?"

**Files:** `src/main.rs` (enhance `--dry-run` path), `src/config/role.rs` (add `resolve_full()` that expands extends/include/variables).

**12C Design — Port Signatures:**

```bash
$ aichat --list-roles --verbose
  code-reviewer    in: text      out: json{issues, severity}    3 tools   extends: base-analyst
  summarizer       in: text      out: text                      0 tools
  classifier       in: json{...} out: json{label, confidence}   0 tools   pipeline: 2 stages
```

Derived from existing `input_schema`/`output_schema`. A one-line human-readable summary of JSON Schema top-level properties. When no schema is defined, shows `in: any, out: text`.

**Files:** `src/config/role.rs` (add `port_signature()` method), `src/main.rs` or `src/config/mod.rs` (render in list output).
