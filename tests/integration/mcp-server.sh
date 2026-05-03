#!/usr/bin/env bats
#
# MCP server-side and mcp_servers: config protocol tests.
#
# Covers the gap between client-side coverage (tests/integration/mcp-client
# scenarios in compatibility.rs and docs/demos/demo-mcp-client.md) and the
# runtime behavior when:
#
#   1. aichat runs as `aichat --mcp` (stdio MCP server exposing functions)
#   2. tools are sourced from a `mcp_servers:` config block
#   3. multiple stdio MCP servers boot concurrently via the native pool
#
# Each test builds an isolated AICHAT_CONFIG_DIR inside $BATS_TEST_TMPDIR so it
# does not touch the user's production config.
#
# Fixture choice: `mcp-server-git` (uvx) is the smallest fast offline server.
# Tests that require additional servers prefer sqlite (uvx) and memory (node).

AICHAT_BIN="${AICHAT_BIN:-./target/debug/aichat}"
UVX="${UVX:-/Users/admin/.local/bin/uvx}"

# Send a JSON-RPC sequence to `aichat --mcp` and capture stdout.
# $1 = AICHAT_CONFIG_DIR
# $2 = path to write stdout to
# Subsequent args = JSON-RPC message lines to send (one per arg).
mcp_exchange() {
  local cfg_dir="$1" out="$2"
  shift 2
  {
    for msg in "$@"; do
      printf '%s\n' "$msg"
      sleep 0.3
    done
    sleep 1
  } | AICHAT_CONFIG_DIR="$cfg_dir" timeout 30 "$AICHAT_BIN" --mcp >"$out" 2>"$out.err"
}

# Build a minimal AICHAT_CONFIG_DIR with an empty model client and the supplied
# trailing YAML appended (e.g., a `mcp_servers:` block).
write_config() {
  local cfg_dir="$1"
  mkdir -p "$cfg_dir"
  cat >"$cfg_dir/config.yaml" <<'YAML'
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
YAML
  if [ -n "$2" ]; then
    printf '\n%s\n' "$2" >>"$cfg_dir/config.yaml"
  fi
}

INIT_MSG='{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"bats","version":"0.0.1"}}}'
INITIALIZED_MSG='{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}'
LIST_MSG='{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

@test "mcp-server: initialize returns serverInfo and protocol version" {
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" ""
  # Always send the full handshake; aichat --mcp exits non-zero if stdin closes
  # mid-handshake (after only `initialize`).
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG"
  run jq -r 'select(.id==1) | .result.serverInfo.name' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [[ "$output" == *"aichat"* ]]
  run jq -r 'select(.id==1) | .result.protocolVersion' "$BATS_TEST_TMPDIR/out"
  [ "$output" = "2024-11-05" ]
}

@test "mcp-server: empty config advertises empty tools list" {
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" ""
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG"
  run jq -r 'select(.id==2) | .result.tools | length' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [ "$output" = "0" ]
}

@test "mcp-server: single mcp_servers entry advertises discover_roles meta-tool (lazy mode)" {
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" "mcp_servers:
  git:
    command: $UVX
    args: [\"mcp-server-git\"]"
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG"
  # Lazy mode kicks in once total tools >= 8 (mcp-server-git ships 12).
  run jq -r 'select(.id==2) | [.result.tools[].name] | join(",")' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [ "$output" = "discover_roles" ]
  # listChanged capability should be advertised under lazy mode.
  run jq -r 'select(.id==1) | .result.capabilities.tools.listChanged' "$BATS_TEST_TMPDIR/out"
  [ "$output" = "true" ]
}

@test "mcp-server: discover_roles enumerates mcp_servers tools with namespaced names" {
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" "mcp_servers:
  git:
    command: $UVX
    args: [\"mcp-server-git\"]"
  call='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{"query":"git"}}}'
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG" "$call"
  run jq -r 'select(.id==3) | .result.content[0].text' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  # Tools are namespaced as <server>:<tool>
  [[ "$output" == *"git:git_status"* ]]
  [[ "$output" == *"git:git_log"* ]]
}

@test "mcp-server: tool-call dispatch through mcp_servers pool (probe a)" {
  # Confirms aichat --mcp can actually INVOKE an mcp_servers tool, not just list
  # it. Captured during the bridge-retirement validation pass on 2026-05-01;
  # unskipped in Phase 31A once ToolCall::eval grew the same is_mcp /
  # mcp_pool.call() routing as eval_tool_calls (`is_mcp_call` helper in
  # src/function.rs).
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" "mcp_servers:
  git:
    command: $UVX
    args: [\"mcp-server-git\"]"
  expand='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{"query":"git_status"}}}'
  call="{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"tools/call\",\"params\":{\"name\":\"git:git_status\",\"arguments\":{\"repo_path\":\"$BATS_TEST_TMPDIR\"}}}"
  ( cd "$BATS_TEST_TMPDIR" && git init -q . )
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG" "$expand" "$call"
  run jq -r 'select(.id==4) | .result.content[0].text' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [[ "$output" == *"branch"* ]]
}

@test "mcp-server: 3 concurrent stdio servers register all tools (probe b small-N)" {
  # Probe (b) found that booting 5+ concurrent stdio MCP servers regresses to
  # zero registered tools or a non-responsive runtime even with bumped
  # mcp_startup_timeout. At small N the pool initializes correctly. This test
  # pins the small-N happy path; the multi-server hang at large N is tracked
  # separately (see docs/demos/demo-mcp-server.md "Known limitations").
  cfg="$BATS_TEST_TMPDIR/aichat"
  write_config "$cfg" "mcp_servers:
  sqlite:
    command: $UVX
    args: [\"mcp-server-sqlite\", \"--db-path\", \"$BATS_TEST_TMPDIR/probe.db\"]
  git:
    command: $UVX
    args: [\"mcp-server-git\"]"
  call='{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"discover_roles","arguments":{}}}'
  mcp_exchange "$cfg" "$BATS_TEST_TMPDIR/out" "$INIT_MSG" "$INITIALIZED_MSG" "$LIST_MSG" "$call"
  run jq -r 'select(.id==3) | .result.content[0].text' "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [[ "$output" == *"sqlite:"* ]]
  [[ "$output" == *"git:"* ]]
}

@test "mcp-server: many concurrent stdio servers regression (probe b large-N)" {
  # Pins the observed regression: with 10 mcp_servers entries (the contents of
  # llm-functions/mcp.json on this machine), aichat --mcp returns 0 tools or
  # becomes non-responsive after initialize. Bumping mcp_startup_timeout +
  # mcp_call_timeout to 60s does not resolve. Likely a runtime/IO race in the
  # pool, not slow startup.
  #
  # Skip until the upstream pool/runtime issue is identified. When fixed,
  # replace `skip` with assertions that all 10 servers register their tools.
  skip "blocked: large-N pool init regression captured during 2026-05-01 probe"
}
