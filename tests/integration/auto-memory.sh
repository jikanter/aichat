#!/usr/bin/env bats

# Phase 34A: Auto-memory read surface.
#   34A  read `memory/MEMORY.md` at startup, cap at 200 lines / 8 KiB, inject
#        the capped content into the system prompt and surface it in
#        `aichat --info -o json` as `memory_preamble`.
#
# All checks are offline: `--info` reads config and the memory file but makes
# no model request. The memory directory is isolated per-test via the
# AICHAT_MEMORY_DIR override so the repo's own `memory/MEMORY.md` never leaks
# into the assertions.

AICHAT_BIN="${AICHAT_BIN:-./target/debug/aichat}"

setup() {
  MEM_DIR="$(mktemp -d)"
  export AICHAT_MEMORY_DIR="$MEM_DIR"
}

teardown() {
  rm -rf "$MEM_DIR"
  unset AICHAT_MEMORY_DIR
}

# ----- 34A: presence in --info -o json -----

@test "auto-memory: MEMORY.md content surfaces in --info -o json" {
  cat > "$MEM_DIR/MEMORY.md" <<EOF
- [Cite sources](feedback_cite_sources.md) — link docs inline
EOF
  run "$AICHAT_BIN" --info -o json
  [ "$status" -eq 0 ]
  [[ "$output" == *"memory_preamble"* ]]
  [[ "$output" == *"Cite sources"* ]]
}

@test "auto-memory: no memory_preamble key when MEMORY.md is absent" {
  run "$AICHAT_BIN" --info -o json
  [ "$status" -eq 0 ]
  [[ "$output" != *"memory_preamble"* ]]
}

@test "auto-memory: empty MEMORY.md yields no preamble" {
  : > "$MEM_DIR/MEMORY.md"
  run "$AICHAT_BIN" --info -o json
  [ "$status" -eq 0 ]
  [[ "$output" != *"memory_preamble"* ]]
}

# ----- 34A: truncation warning -----

@test "auto-memory: truncation warning fires past the 200-line cap" {
  # 250 numbered lines — exceeds the 200-line cap.
  for i in $(seq 1 250); do
    echo "- memory line $i" >> "$MEM_DIR/MEMORY.md"
  done
  run "$AICHAT_BIN" --info -o json
  [ "$status" -eq 0 ]
  # Warning lands on stderr; bats `run` folds stderr into $output.
  [[ "$output" == *"memory preamble cap"* ]]
  [[ "$output" == *"split"* ]]
  # The first 200 lines survive the cap; 201..250 are dropped.
  [[ "$output" == *"memory line 200"* ]]
  [[ "$output" != *"memory line 201"* ]]
  [[ "$output" != *"memory line 250"* ]]
}

@test "auto-memory: under-cap file fires no warning" {
  printf -- '- one\n- two\n- three\n' > "$MEM_DIR/MEMORY.md"
  run "$AICHAT_BIN" --info -o json
  [ "$status" -eq 0 ]
  [[ "$output" != *"memory preamble cap"* ]]
}
