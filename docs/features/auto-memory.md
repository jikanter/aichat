# Auto-Memory (`memory/MEMORY.md`)

**Status:** 34A shipped (read-only). The write loop (34C/34D) is deferred —
see [`docs/roadmap/phase-34-overview.md`](../roadmap/phase-34-overview.md).

Auto-memory is aichat's freeform, read-on-startup notes layer. It is the
session-level counterpart to the typed [`knowledge/`](knowledge.md) store:
where `knowledge/` holds citable, deduplicated facts compiled from source
files, `memory/` holds incidental learnings a human jots down by hand —
preferences expressed mid-conversation, project conventions, reminders. The
two stores never merge and never silently promote into one another.

## What 34A does

At startup aichat reads `memory/MEMORY.md` (if present), caps it, and injects
the capped content into the **system prompt**, appended after the active
role's prompt body. It is **read-only**: aichat never writes to `memory/` in
this phase.

This mirrors Claude Code's first-~200-lines auto-load discipline: a small
standing-context file the agent always sees, kept short enough not to crowd
the role's own instructions.

## Discovery precedence

The first match wins; the stores never merge (phase-34 open question 1):

1. **`AICHAT_MEMORY_DIR`** — explicit override. If set, only this directory is
   consulted; no fallback. Used by tests and by power users who keep memory
   outside the default chain.
2. **Project-local** — `./memory/MEMORY.md` relative to the working directory.
   Each project carries its own memory, matching the `knowledge/` precedent.
3. **User-level** — `<config_dir>/memory/MEMORY.md` (e.g.
   `~/.config/aichat/memory/MEMORY.md`). The global preference layer,
   equivalent to `~/.claude/CLAUDE.md`.

An absent or empty `MEMORY.md` is a clean no-op — no system-prompt change and
zero added tokens.

## The cap

The preamble is capped at **200 lines or 8 KiB, whichever hits first** (Claude
Code parity; phase-34 open question 3). When the cap drops content, aichat
emits a one-line warning to stderr:

```
warning: <path>/MEMORY.md exceeds the 200-line / 8-KiB memory preamble cap;
         split it into topic files so context is not dropped
```

The cap never splits a UTF-8 character: it drops whole trailing lines first,
and hard-truncates a lone over-budget line on a char boundary.

## Inspecting the loaded preamble

The preamble is observable without a model call via `--info`:

```bash
aichat --info -o json        # -> { ..., "memory_preamble": "# Project memory\n- ..." }
aichat --info                # -> a `memory_preamble  <N> chars` row
```

The injected block is framed with a `# Project memory` header so the model
reads it as standing context rather than task instructions. `--info` shows the
raw memory; the `--dry-run` role preview does **not** include it (dry-run
previews the role file, not the assembled messages).

## Which surfaces inject memory

| Surface | Injection point |
|---|---|
| `aichat "..."`, `aichat -r <role>`, `aichat -a <agent>` | Rust `Input::build_messages` (`src/config/input.rs`) |
| Legacy built-in REPL | same Rust path |
| HTTP server, role path (`/v1/chat/completions` with a role) | same Rust path |
| **pi REPL — native agent turns** | `before_agent_start` hook in the bundled pi extension (`assets/pi-extensions/aichat-bridge.js`) |

Pi's native turns build their own system prompt independent of any aichat
role, so the pi extension carries a matching reader capped to the same
200-line / 8-KiB budget. The OpenAI-compatible passthrough path (raw
`messages`, no role) is intentionally **not** injected — those requests carry
their own system prompt and may originate from external clients.

## Relationship to `knowledge/`

| | `memory/` (this feature) | `knowledge/` (Phases 25–27) |
|---|---|---|
| Storage | Markdown + YAML frontmatter, freeform | Typed JSONL + `manifest.yaml` |
| Writer | Human (by hand); Reflector in 34C (deferred) | `--knowledge-reflect` / `--knowledge-curate` |
| Query | Read on startup; lazy topic-load in 34B (reserved) | Tag → BM25 → graph-walk → RRF |
| Audit | Git history | Append-only `revisions.jsonl` |

## Deferred (34B–34D)

- **34B** — lazy-load of topic files referenced from `MEMORY.md`. The loader
  API is reserved; its reference sources depend on other Epic-14 themes.
- **34C** — session-exit Reflector emitting candidate `memory/<topic>.md`
  files. Gated on a separate design review of the Reflector prompt and the
  secret-redaction pass.
- **34D** — Curator gate (interactive accept/skip/edit, `--memory-auto-curate`).
  Lands with 34C so no candidate is written without a gate.

See the [demo](../demos/phase-34-auto-memory.md) for runnable examples.