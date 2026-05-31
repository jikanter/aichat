# Phase 34A: Auto-Memory Read Surface

*2026-05-30T22:29:04Z by Showboat 0.6.1*
<!-- showboat-id: 2813c259-060c-4b6c-b873-48e02f915976 -->

Phase 34A wires a read-only `memory/MEMORY.md` surface. At startup aichat reads the project-local `memory/MEMORY.md` (env override `AICHAT_MEMORY_DIR` for isolation), caps it to 200 lines / 8 KiB, and injects the capped block into the system prompt after the active role's prompt body. The preamble is observable — token-free — via `aichat --info -o json`.

**Setup:** an isolated memory directory with a small MEMORY.md index.

```bash
rm -rf /tmp/p34-demo-mem && mkdir -p /tmp/p34-demo-mem
printf -- '- [Cite sources](feedback_cite_sources.md) — link docs inline in code\n- [Prefer tokio](rust_async.md) — standardize on tokio across the codebase\n' > /tmp/p34-demo-mem/MEMORY.md
cat /tmp/p34-demo-mem/MEMORY.md
```

```output
- [Cite sources](feedback_cite_sources.md) — link docs inline in code
- [Prefer tokio](rust_async.md) — standardize on tokio across the codebase
```

**34A — preamble surfaces in `--info -o json`.** The capped MEMORY.md, framed with a `# Project memory` header, appears under `memory_preamble` without any model call.

```bash
AICHAT_MEMORY_DIR=/tmp/p34-demo-mem ./target/debug/aichat --info -o json | python3 -c 'import sys,json; print(json.load(sys.stdin)["memory_preamble"])'
```

```output
# Project memory
- [Cite sources](feedback_cite_sources.md) — link docs inline in code
- [Prefer tokio](rust_async.md) — standardize on tokio across the codebase
```

**34A — the 200-line / 8-KiB cap.** A MEMORY.md past the cap is truncated and a one-line warning is emitted to stderr nudging the user to split into topic files. Here a 250-line file keeps the first 200 lines; line 201+ are dropped.

```bash
seq 1 250 | sed 's/^/- memory line /' > /tmp/p34-demo-mem/MEMORY.md
AICHAT_MEMORY_DIR=/tmp/p34-demo-mem ./target/debug/aichat --info -o json 2>/tmp/p34-warn.txt >/tmp/p34-info.json
echo '# stderr warning:'
cat /tmp/p34-warn.txt
echo '# last kept line / first dropped line:'
python3 -c 'import json; t=json.load(open("/tmp/p34-info.json"))["memory_preamble"]; print("line 200 present:", "memory line 200" in t); print("line 201 present:", "memory line 201" in t)'
```

```output
# stderr warning:
warning: /tmp/p34-demo-mem/MEMORY.md exceeds the 200-line / 8-KiB memory preamble cap; split it into topic files so context is not dropped
# last kept line / first dropped line:
line 200 present: True
line 201 present: False
```

**34A — absent / empty MEMORY.md is a clean no-op.** No memory directory means no `memory_preamble` key and zero added tokens.

```bash
rm -rf /tmp/p34-demo-mem
AICHAT_MEMORY_DIR=/tmp/p34-demo-mem ./target/debug/aichat --info -o json | python3 -c 'import sys,json; d=json.load(sys.stdin); print("memory_preamble" in d)'
```

```output
False
```

**Surfaces covered.** The Rust loader injects memory for role/agent/prompt turns (`aichat "..."`, `-r`, `-a`, the legacy REPL, and the server's role path) at `Input::build_messages`. Pi's *native* agent turns — which build their own system prompt independent of any aichat role — are covered by the matching `before_agent_start` hook in the bundled pi extension (`assets/pi-extensions/aichat-bridge.js`), capped to the same budget. Read-only throughout: the 34C/34D write loop (Reflector + Curator) is deferred pending a separate design review.
