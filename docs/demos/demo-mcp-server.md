# MCP Server-Side: aichat --mcp and mcp_servers

*2026-05-01T16:59:50Z by Showboat 0.6.1*
<!-- showboat-id: 4d0ff528-06fd-4831-a3b9-1408e4d6125b -->

aichat exposes its functions and MCP-pool tools to external clients via `aichat --mcp` (stdio MCP server). External MCP servers configured under `mcp_servers:` in `config.yaml` are loaded into the same pool and re-advertised under namespaced names like `<server>:<tool>`.

This demo walks the protocol end-to-end and pins two regressions captured during the 2026-05-01 bridge-retirement validation pass.

## 0. Setup

The exec blocks below share a tiny probe driver that builds an isolated `AICHAT_CONFIG_DIR` per call, writes a supplied YAML, and pipes JSON-RPC into `aichat --mcp`. They also share three YAML fixtures for the empty / git-only / git+sqlite scenarios. The setup block writes them to `/tmp` so `showboat verify` can re-run the demo end-to-end.

```bash
cat >/tmp/aichat_mcp_probe.sh <<'PROBE'
#!/usr/bin/env bash
set -e
yaml="$1"
cfg="$(mktemp -d)/aichat"
mkdir -p "$cfg"
cp "$yaml" "$cfg/config.yaml"
{
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    printf "%s\n" "$line"
    sleep 0.3
  done
  sleep 1
} | AICHAT_CONFIG_DIR="$cfg" timeout 30 "${AICHAT_BIN:-aichat}" --mcp 2>/dev/null
rm -rf "$cfg"
PROBE
chmod +x /tmp/aichat_mcp_probe.sh
cat >/tmp/aichat_mcp_empty.yaml <<EMPTY
model: ollama:gemma4:26b
function_calling: true
clients:
- type: openai-compatible
  name: ollama
  api_base: http://localhost:11434/v1
  models:
    - name: gemma4:26b
      max_input_tokens: 160000
      max_output_tokens: 8942
      supports_function_calling: true
EMPTY
cat >/tmp/aichat_mcp_git.yaml <<GITONLY
model: ollama:gemma4:26b
function_calling: true
clients:
- type: openai-compatible
  name: ollama
  api_base: http://localhost:11434/v1
  models:
    - name: gemma4:26b
      max_input_tokens: 160000
      max_output_tokens: 8942
      supports_function_calling: true

mcp_servers:
  git:
    command: /Users/admin/.local/bin/uvx
    args: ["mcp-server-git"]
GITONLY
cat >/tmp/aichat_mcp_small_n.yaml <<SMALLN
model: ollama:gemma4:26b
function_calling: true
clients:
- type: openai-compatible
  name: ollama
  api_base: http://localhost:11434/v1
  models:
    - name: gemma4:26b
      max_input_tokens: 160000
      max_output_tokens: 8942
      supports_function_calling: true

mcp_servers:
  sqlite:
    command: /Users/admin/.local/bin/uvx
    args: ["mcp-server-sqlite", "--db-path", "/tmp/probe-sqlite-demo.db"]
  git:
    command: /Users/admin/.local/bin/uvx
    args: ["mcp-server-git"]
SMALLN
echo setup-ok
```

```output
setup-ok
```

## 1. Initialize handshake (empty config)

With no functions and no `mcp_servers:` block, aichat initializes cleanly and advertises an empty tool list.

```bash
printf '%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_empty.yaml | jq -c 'select(.id==1) | {protocol: .result.protocolVersion, server: .result.serverInfo.name, tools_capability: .result.capabilities.tools}'
```

```output
{"protocol":"2024-11-05","server":"aichat","tools_capability":{}}
```

```bash
printf '%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_empty.yaml | jq -c 'select(.id==2) | {advertised_tool_count: (.result.tools | length)}'
```

```output
{"advertised_tool_count":0}
```

## 2. Adding an mcp_servers entry

Add one stdio MCP server (`mcp-server-git`) under `mcp_servers:`. aichat's native MCP client connects, the pool registers all 12 git tools, and lazy mode kicks in (threshold = 8), so only the `discover_roles` meta-tool is initially advertised.

```bash
printf '%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_git.yaml | jq -c 'select(.id==1) | .result.capabilities.tools'
```

```output
{"listChanged":true}
```

```bash
printf '%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_git.yaml | jq -c 'select(.id==2) | [.result.tools[].name]'
```

```output
["discover_roles"]
```

## 3. discover_roles surfaces the namespaced tool list

The `discover_roles` meta-tool returns a flat description of every tool in the pool. `mcp_servers:` tools are namespaced as `<server>:<tool>` to avoid collisions.

```bash
printf '%s\n%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{"query":"git_"}}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_git.yaml | jq -r 'select(.id==3) | .result.content[0].text' | grep -E '^- git:' | sort | head -6
```

```output
- git:git_add: Adds file contents to the staging area
- git:git_branch: List Git branches
- git:git_checkout: Switches branches
- git:git_commit: Records changes to the repository
- git:git_create_branch: Creates a new branch from an optional base branch
- git:git_diff_staged: Shows changes that are staged for commit
```

## 4. Known limitation: tool-call dispatch (probe a)

aichat exposes `mcp_servers:` tools but, as of 0.5.1-eridian, cannot dispatch them through `--mcp` mode. Single-call `ToolCall::eval` (`function.rs:337`) lacks the MCP-pool routing branch that the batch path `eval_tool_calls` has (`function.rs:33-44`), so calls fall through to the llm-functions binary lookup and fail.

```bash
printf '%s\n%s\n%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{"query":"git_"}}}' '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"git:git_status","arguments":{"repo_path":"/tmp"}}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_git.yaml | jq -r 'select(.id==4) | .error.message // "(unexpected success)"' | grep -oE 'binary not found' | head -1
```

```output
binary not found
```

The "binary not found" hint confirms the call took the llm-functions binary path instead of the MCP pool. The fix is to port the `is_mcp` check from `eval_tool_calls` into `ToolCall::eval`.

## 5. Multi-server pool happy path (probe b small-N)

Two concurrent stdio servers (`sqlite` + `git`) boot cleanly and register all of their tools. The large-N regression (10 servers from the production `mcp.json`) is pinned by a `skip`-marked test in `tests/integration/mcp-server.sh` until the pool/runtime issue is identified.

```bash
printf '%s\n%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{}}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_small_n.yaml | jq -r 'select(.id==3) | .result.content[0].text' | grep -oE '^- (sqlite|git):[a-z_]+' | awk -F: '{print $2}' | sort -u | head -1
```

```output
append_insight
```

```bash
printf '%s\n%s\n%s\n%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"demo","version":"0"}}}' '{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}' '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{}}}' | /tmp/aichat_mcp_probe.sh /tmp/aichat_mcp_small_n.yaml | jq -r 'select(.id==3) | .result.content[0].text' | grep -cE '^- (sqlite|git):'
```

```output
18
```

## Verification

```bash
showboat verify docs/demos/demo-mcp-server.md
```

The same protocol is encoded as repeatable bats tests in `tests/integration/mcp-server.sh`.
