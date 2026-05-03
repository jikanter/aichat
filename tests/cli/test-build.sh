#!/usr/bin/env bats

PROJECT_DIR="${PROJECT_DIR:-"${HOME}/Developer/Projects/aichat"}"

test-local-server() {
  cd "${PROJECT_DIR}" || exit 1;
  result=$(argc models-openai-compatible --api-base http://localhost:8001/v1 --api-key="" |jq '.data[0].owned_by')
  [ -n "$result" ]
}