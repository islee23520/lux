#!/usr/bin/env bash
# e2e-lux-sequential-smoke.sh — Exercise the LUX sequential loop lifecycle.
#
# This smoke test creates an isolated temporary project, runs the real compiled
# gateway CLI/server, writes a valid Lux spec through HTTP, then drives the loop
# from idle through analysis, approval-gated refinement/build/play, feedback,
# and shutdown. Unity is not required: the current gateway build step queues a
# WebGL build job without launching Unity.
#
# Prerequisites:
#   - cargo-built gateway binary at gateway/target/debug/lux or target/release/lux
#   - curl
#   - jq optional; when unavailable, a sed-based extractor is used for simple fields
#
# Usage:
#   bash scripts/e2e-lux-sequential-smoke.sh [--quick]
#   --quick  Use shorter server startup waits and skip optional endpoint probes.

set -Eeuo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
GATEWAY_DIR="$ROOT_DIR/gateway"
QUICK=false

if [[ "${1:-}" == "--quick" ]]; then
  QUICK=true
elif [[ "${1:-}" == "--help" || "${1:-}" == "-h" ]]; then
  sed -n '2,20p' "$0"
  exit 0
elif [[ -n "${1:-}" ]]; then
  echo "FAIL unknown argument: $1" >&2
  exit 1
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

TMP_DIR=""
PROJECT_DIR=""
SERVER_PID=""
SERVER_LOG=""
LAST_BODY=""
LAST_STATUS=""
RESPONSE_BODY=""
APPROVE_STATE=""
LOOP_STARTED=false
RUN_STATE_DIRECT_SUPPORTED=unknown

cleanup() {
  local exit_code=$?
  if [[ -n "$SERVER_PID" ]] && kill -0 "$SERVER_PID" >/dev/null 2>&1; then
    kill "$SERVER_PID" >/dev/null 2>&1 || true
    wait "$SERVER_PID" >/dev/null 2>&1 || true
  fi
  if [[ -n "$TMP_DIR" && -d "$TMP_DIR" ]]; then
    rm -rf "$TMP_DIR"
  fi
  exit "$exit_code"
}
trap cleanup EXIT INT TERM

pass() {
  printf "  ${GREEN}PASS${NC} %s\n" "$1"
}

note() {
  printf "  ${YELLOW}NOTE${NC} %s\n" "$1"
}

fail() {
  printf "  ${RED}FAIL${NC} %s\n" "$1" >&2
  if [[ -n "${LAST_STATUS:-}" ]]; then
    printf "  HTTP status: %s\n" "$LAST_STATUS" >&2
  fi
  if [[ -n "${LAST_BODY:-}" ]]; then
    printf "  Response: %s\n" "$(printf '%s' "$LAST_BODY" | tr '\n' ' ' | cut -c 1-1200)" >&2
  fi
  if [[ -n "$SERVER_LOG" && -f "$SERVER_LOG" ]]; then
    printf "  Server log: %s\n" "$SERVER_LOG" >&2
    tail -n 40 "$SERVER_LOG" >&2 || true
  fi
  exit 1
}

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "required command not found: $1"
}

find_lux_bin() {
  if [[ -n "${LUX_BIN:-}" ]]; then
    [[ -x "$LUX_BIN" ]] || fail "LUX_BIN is not executable: $LUX_BIN"
    printf '%s\n' "$LUX_BIN"
    return
  fi
  for candidate in \
    "$GATEWAY_DIR/target/debug/lux" \
    "$GATEWAY_DIR/target/release/lux" \
    "$GATEWAY_DIR/target/debug/gateway" \
    "$GATEWAY_DIR/target/release/gateway" \
    "$ROOT_DIR/target/debug/lux" \
    "$ROOT_DIR/target/release/lux" \
    "$ROOT_DIR/target/debug/gateway" \
    "$ROOT_DIR/target/release/gateway"; do
    if [[ -x "$candidate" ]]; then
      printf '%s\n' "$candidate"
      return
    fi
  done
  fail "gateway binary not found; run: cd gateway && cargo build"
}

json_escape() {
  local value=$1
  value=${value//\\/\\\\}
  value=${value//\"/\\\"}
  value=${value//$'\n'/\\n}
  value=${value//$'\r'/\\r}
  value=${value//$'\t'/\\t}
  printf '%s' "$value"
}

json_get() {
  local json=$1
  local path=$2
  if command -v jq >/dev/null 2>&1; then
    printf '%s' "$json" | jq -er "$path" 2>/dev/null || return 1
    return 0
  fi

  case "$path" in
    .ok|.valid)
      printf '%s' "$json" | sed -nE 's/.*"'"${path#.}"'"[[:space:]]*:[[:space:]]*(true|false).*/\1/p' | head -n 1
      ;;
    .status|.state|.loop.state|.loop.iteration|.iteration|.requiresUserApproval|.pendingState|.approvalGate)
      local key="${path##*.}"
      printf '%s' "$json" | sed -nE 's/.*"'"$key"'"[[:space:]]*:[[:space:]]*"?([^",}\]]+)"?.*/\1/p' | head -n 1
      ;;
    .schemaVersion|.schema_version|.schemaVersion?//.schema_version)
      printf '%s' "$json" | sed -nE 's/.*"(schemaVersion|schema_version)"[[:space:]]*:[[:space:]]*([0-9]+).*/\2/p' | head -n 1
      ;;
    .projectName|.project_name|.projectName?//.project_name)
      printf '%s' "$json" | sed -nE 's/.*"(projectName|project_name)"[[:space:]]*:[[:space:]]*"([^"]+)".*/\2/p' | head -n 1
      ;;
    *)
      return 1
      ;;
  esac
}

assert_eq() {
  local actual=$1
  local expected=$2
  local description=$3
  [[ "$actual" == "$expected" ]] || fail "$description: expected '$expected', got '$actual'"
}

assert_one_of() {
  local actual=$1
  local description=$2
  shift 2
  local expected
  for expected in "$@"; do
    if [[ "$actual" == "$expected" ]]; then
      return 0
    fi
  done
  fail "$description: unexpected value '$actual'"
}

request_json() {
  local method=$1
  local path=$2
  local body=${3:-}
  local response_file="$TMP_DIR/response.json"
  local url="http://127.0.0.1:$PORT$path"
  local status

  if [[ -n "$body" ]]; then
    status=$(curl -sS -o "$response_file" -w '%{http_code}' \
      -X "$method" \
      -H 'content-type: application/json' \
      --data "$body" \
      "$url") || fail "$method $path failed to connect"
  else
    status=$(curl -sS -o "$response_file" -w '%{http_code}' \
      -X "$method" \
      "$url") || fail "$method $path failed to connect"
  fi

  LAST_STATUS="$status"
  LAST_BODY="$(cat "$response_file")"
  RESPONSE_BODY="$LAST_BODY"
}

request_project_get() {
  local path=$1
  local response_file="$TMP_DIR/response.json"
  local status
  status=$(curl -sS -G -o "$response_file" -w '%{http_code}' \
    --data-urlencode "project_path=$PROJECT_DIR" \
    "http://127.0.0.1:$PORT$path") || fail "GET $path failed to connect"
  LAST_STATUS="$status"
  LAST_BODY="$(cat "$response_file")"
  RESPONSE_BODY="$LAST_BODY"
}

assert_http_ok() {
  local description=$1
  case "$LAST_STATUS" in
    200|201) pass "$description" ;;
    *) fail "$description" ;;
  esac
}

wait_for_server() {
  local attempts=60
  if [[ "$QUICK" == true ]]; then
    attempts=25
  fi
  local i body ok
  for ((i = 1; i <= attempts; i++)); do
    if request_json GET /api/health "" 2>/dev/null; then
      body="$RESPONSE_BODY"
      ok=$(json_get "$body" '.ok' || true)
      if [[ "$LAST_STATUS" == "200" && "$ok" == "true" ]]; then
        pass "Step 2: lux serve started and /api/health ok=true"
        return 0
      fi
    fi
    sleep 0.2
  done
  fail "Step 2: lux serve did not become healthy"
}

run_state_body() {
  local body status value

  if [[ "$RUN_STATE_DIRECT_SUPPORTED" != "no" && "$QUICK" != true ]]; then
    request_project_get /api/run-state || true
    body="$RESPONSE_BODY"
    status="$LAST_STATUS"
    if [[ "$status" == "200" ]]; then
      RUN_STATE_DIRECT_SUPPORTED=yes
      return 0
    fi
    RUN_STATE_DIRECT_SUPPORTED=no
    note "/api/run-state returned HTTP $status; using current gateway run-state view endpoints"
  fi

  if [[ "$LOOP_STARTED" == true ]]; then
    request_json GET /api/lux/loop/status ""
    body="$RESPONSE_BODY"
    [[ "$LAST_STATUS" == "200" ]] || fail "GET /api/lux/loop/status"
    RESPONSE_BODY="$body"
    return 0
  fi

  request_project_get /api/lux/progress/summary
  body="$RESPONSE_BODY"
  [[ "$LAST_STATUS" == "200" ]] || fail "GET /api/lux/progress/summary"
  value=$(json_get "$body" '.loop.state' || true)
  [[ -n "$value" ]] || fail "progress summary missing loop.state"
  RESPONSE_BODY="$body"
}

extract_state() {
  local body=$1
  local state
  state=$(json_get "$body" '.status' || true)
  [[ "$state" == "null" ]] && state=""
  if [[ -z "$state" ]]; then
    state=$(json_get "$body" '.state' || true)
    [[ "$state" == "null" ]] && state=""
  fi
  if [[ -z "$state" ]]; then
    state=$(json_get "$body" '.loop.state' || true)
    [[ "$state" == "null" ]] && state=""
  fi
  printf '%s' "$state"
}

approve_next() {
  local body state
  request_json POST /api/lux/loop/approve '{"approved":true}'
  body="$RESPONSE_BODY"
  assert_http_ok "$1"
  state=$(extract_state "$body")
  [[ -n "$state" ]] || fail "$1: response missing state"
  APPROVE_STATE="$state"
}

advance_to_awaiting_play() {
  local state=""
  local i
  for ((i = 1; i <= 4; i++)); do
    approve_next "approval gate $i advanced loop"
    state="$APPROVE_STATE"
    if [[ "$state" == "AwaitingPlay" ]]; then
      pass "Step 6b: loop reached AwaitingPlay before simulated play"
      return 0
    fi
    assert_one_of "$state" "approval gate $i transition" "SpecRefining" "Building" "AwaitingPlay" "Idle"
  done
  fail "loop did not reach AwaitingPlay after approval gates; last state '$state'"
}

require_cmd curl
LUX_BIN_RESOLVED="$(find_lux_bin)"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/lux-sequential-smoke.XXXXXX")"
PROJECT_DIR="$TMP_DIR/project"
SERVER_LOG="$TMP_DIR/lux-serve.log"
mkdir -p "$PROJECT_DIR"
PORT=$((45000 + RANDOM % 10000))

echo "LUX sequential smoke"
echo "  binary:  $LUX_BIN_RESOLVED"
echo "  project: $PROJECT_DIR"
echo "  port:    $PORT"
if ! command -v jq >/dev/null 2>&1; then
  note "jq not found; using sed fallback for simple JSON assertions"
fi

"$LUX_BIN_RESOLVED" --no-update-check init --project-path "$PROJECT_DIR" --no-interactive >/dev/null 2>&1 \
  || fail "Step 1: lux init failed"
[[ -d "$PROJECT_DIR/.lux" ]] || fail "Step 1: .lux directory was not created"
[[ -f "$PROJECT_DIR/.lux/spec.json" ]] || fail "Step 1: .lux/spec.json was not created"
pass "Step 1: lux init created isolated .lux workspace"

"$LUX_BIN_RESOLVED" --no-update-check serve \
  --host 127.0.0.1 \
  --port "$PORT" \
  --project-path "$PROJECT_DIR" \
  --idle-timeout 0 \
  >"$SERVER_LOG" 2>&1 &
SERVER_PID=$!
wait_for_server

request_project_get /api/lux/spec
spec_body="$RESPONSE_BODY"
assert_http_ok "loaded initialized spec over HTTP"
schema_version=$(json_get "$spec_body" '.schema_version' || true)
[[ -n "$schema_version" ]] || fail "initialized spec response missing schema_version"

escaped_project_path="$(json_escape "$PROJECT_DIR")"
put_body=$(printf '{"project_path":"%s","spec":%s}' "$escaped_project_path" "$spec_body")
request_json PUT /api/lux/spec "$put_body"
written_spec="$RESPONSE_BODY"
assert_http_ok "Step 3: PUT /api/lux/spec wrote valid spec.json"
written_name=$(json_get "$written_spec" '.project_name' || true)
[[ -n "$written_name" ]] || fail "Step 3: spec write response missing project_name"

run_state_body
state_body="$RESPONSE_BODY"
state_value=$(extract_state "$state_body")
assert_one_of "$state_value" "Step 4: run-state initializes correctly" "Idle"
pass "Step 4: run-state initializes as Idle"

request_json POST /api/lux/loop/start "$(printf '{"projectPath":"%s","maxIterations":1}' "$escaped_project_path")"
start_body="$RESPONSE_BODY"
assert_http_ok "Step 5: POST /api/lux/loop/start triggered analysis"
LOOP_STARTED=true
start_state=$(extract_state "$start_body")
assert_eq "$start_state" "Analyzing" "Step 5: loop start state"

run_state_body
state_body="$RESPONSE_BODY"
state_value=$(extract_state "$state_body")
assert_eq "$state_value" "Analyzing" "Step 6: run-state after loop start"
pass "Step 6: run-state transitioned to Analyzing"

advance_to_awaiting_play

request_json POST /api/lux/loop/play-started '{}'
play_body="$RESPONSE_BODY"
assert_http_ok "Step 7: POST /api/lux/loop/play-started accepted simulated play start"
play_state=$(extract_state "$play_body")
assert_eq "$play_state" "CollectingFeedback" "Step 7: loop state after play-started"

run_state_body
state_body="$RESPONSE_BODY"
state_value=$(extract_state "$state_body")
assert_eq "$state_value" "CollectingFeedback" "Step 8: run-state after play-started"
pass "Step 8: run-state transitioned to awaiting_feedback equivalent (CollectingFeedback)"

request_json POST /api/lux/loop/feedback '{"rating":5,"text":"sequential smoke feedback","issues":[]}'
feedback_body="$RESPONSE_BODY"
assert_http_ok "Step 9: POST /api/lux/loop/feedback submitted feedback"
feedback_state=$(extract_state "$feedback_body")
assert_one_of "$feedback_state" "Step 9: feedback response state" "Idle" "Updating" "Analyzing" "Completed"

run_state_body
state_body="$RESPONSE_BODY"
state_value=$(extract_state "$state_body")
assert_one_of "$state_value" "Step 10: run-state after feedback" "Idle" "Updating" "Analyzing" "Completed"
pass "Step 10: run-state reached next/finished state ($state_value)"

request_json GET /api/health ""
health_body="$RESPONSE_BODY"
assert_http_ok "Step 11: GET /api/health returned 200"
health_ok=$(json_get "$health_body" '.ok' || true)
assert_eq "$health_ok" "true" "Step 11: health ok"
pass "Step 11: server still healthy"

kill "$SERVER_PID" >/dev/null 2>&1 || true
wait "$SERVER_PID" >/dev/null 2>&1 || true
SERVER_PID=""
rm -rf "$TMP_DIR"
TMP_DIR=""
pass "Step 12: cleanup killed server and removed temp .lux workspace"

echo ""
printf "${GREEN}All LUX sequential smoke steps passed.${NC}\n"
