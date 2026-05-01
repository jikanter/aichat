# Integrated Architecture

This directory holds requirements, plans, and design notes that span more than one project. The current set of integrated systems is:

- **aichat** (this repo) — the CLI / runtime / MCP server-and-client.
- **llm-functions** (`/Volumes/ExternalData/admin/Developer/Scripts/llm-functions`, symlinked to `~/Library/Application Support/aichat/functions`) — tool and agent declarations consumed by aichat.
- **harness interface** — a future surface (TBD) that will let other clients (Claude Code, Cursor, etc.) consume aichat's exposed roles, tools, and MCP-pool servers as a single unit.

A document belongs here when its requirements only make sense across two or more of those systems — e.g., a change in aichat's MCP routing that depends on llm-functions' `tools.txt` registration, or a harness-side feature that needs both aichat and llm-functions to agree on a tool naming scheme.

Documents that live entirely inside one project belong in that project's own roadmap or design directory.

## Index

- [`bridge-retirement.md`](./bridge-retirement.md) — Plan to retire the Node HTTP bridge in `llm-functions/mcp/bridge/` in favor of aichat's native `mcp_servers:` config and `mcp_pool`. Status: blocked on two upstream aichat bugs; tests and demo pinned in aichat to track readiness.
