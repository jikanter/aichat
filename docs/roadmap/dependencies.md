# Cross-Epic Dependency Graph

How the epics relate. Linked from [`ROADMAP.md`](../ROADMAP.md).

```
Epic 1 (Core Platform)         ──── DONE ──────────────────────────────────────────
  │
  ├── Epic 2 (Runtime Intelligence) ─── Phases 9-11, 37 ──── Phase 11D + 37 outstanding
  │     │
  │     ├── Epic 3 (Composition UX) ─── Phase 12 DONE, Phase 13 planned
  │     │     │
  │     │     └── Epic 4 (Typed Ports) ─── Phase 14 DONE, Phase 15 partial
  │     │           │
  │     │           ├── Epic 5 (Server Engine) ─── Phase 16 (F/G/H) + 17 DONE; 18 deferred
  │     │           │     │
  │     │           │     └── Epic 6 (Universal Addressing) ─── Phases 19-20 ──── DONE
  │     │           │           │
  │     │           │           └── Epic 7 (DAG Execution) ─── Phase 21 DONE; 22 planned
  │     │           │
  │     │           └── Epic 8 (Feedback Loop) ─── Phases 23-24 ─── Independent track
  │     │
  │     └── Epic 9 (Knowledge Evolution) ─── Phases 25-27 ──── DONE
  │
  ├── Epic 10 (Entity Evolution) ─── Phases 28-29 ─── Planned
  │
  ├── Epic 11 (Bridge Retirement) ─── Phase 31 ──── DONE
  │
  ├── Epic 12 (Macro Compilation) ─── Phase 30 ──── DONE
  │
  └── Epic 13 (Pi as REPL Surface) ─── Phase 32 ──── DONE
```

**Critical path (active):** Phase 11D → Phase 13 → Phase 15 (B/C) → Phase 22.
Epic 8 (23-24) and Epic 10 (28-29) are independent tracks that can proceed in parallel.
Phase 37 (transparent response caching, Epic 2) is also an independent parallel track — its sequencing is internal (37A → 37B → 37C → 37D → 37E with 37F deferred) and its only cross-phase coupling is to the open-harness trace workstream at 37E (`schema_version` bump for `cache.lookup`). The pi integration is 37D — every pi turn already flows through the in-process `serve.rs`, so wiring the LRU there is transparent.
Phase 18 (server discovery/estimation) remains deferred.
