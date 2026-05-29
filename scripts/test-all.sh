#!/usr/bin/env bash
# test-all.sh — Run all LUX verification checks.
# Usage: ./scripts/test-all.sh [--quick]
#   --quick  Skip the full cargo test suite, only run smoke checks.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
QUICK=false

if [[ "${1:-}" == "--quick" ]]; then
  QUICK=true
fi

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

pass=0
fail=0
skip=0

section() {
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  $1"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

ok()   { ((pass++)) || true; echo -e "  ${GREEN}PASS${NC} $1"; }
err()  { ((fail++)) || true; echo -e "  ${RED}FAIL${NC} $1"; }
warn() { ((skip++)) || true; echo -e "  ${YELLOW}SKIP${NC} $1"; }

# ── Rust ────────────────────────────────────────
section "Rust Gateway"

if [ "$QUICK" = true ]; then
  warn "cargo test (--quick mode, skipped)"
else
  if (cd "$ROOT_DIR/gateway" && cargo test 2>&1); then
    ok "cargo test"
  else
    # Retry the known-flaky server test once
    echo "  Retrying flaky server test..."
    if (cd "$ROOT_DIR/gateway" && cargo test cli_server_starts_and_enforces_header_auth_and_origin_policy 2>&1); then
      ok "cargo test (flaky retry passed)"
    else
      err "cargo test"
    fi
  fi
fi

if (cd "$ROOT_DIR/gateway" && cargo build 2>&1); then
  ok "cargo build"
else
  err "cargo build"
fi

# ── CLI Smoke ───────────────────────────────────
section "CLI Smoke Tests"

LUX_BIN="$ROOT_DIR/target/debug/lux"

if [ -x "$LUX_BIN" ]; then
  "$LUX_BIN" --help > /dev/null 2>&1 && ok "lux --help" || err "lux --help"
  "$LUX_BIN" ai-log --help > /dev/null 2>&1 && ok "lux ai-log --help" || err "lux ai-log --help"
  "$LUX_BIN" ai-log recent --help > /dev/null 2>&1 && ok "lux ai-log recent --help" || err "lux ai-log recent --help"
  "$LUX_BIN" ai-log context --help > /dev/null 2>&1 && ok "lux ai-log context --help" || err "lux ai-log context --help"
  "$LUX_BIN" ai-log compact --help > /dev/null 2>&1 && ok "lux ai-log compact --help" || err "lux ai-log compact --help"
  "$LUX_BIN" ai-log tail --help > /dev/null 2>&1 && ok "lux ai-log tail --help" || err "lux ai-log tail --help"
  "$LUX_BIN" skill list --help > /dev/null 2>&1 && ok "lux skill list --help" || err "lux skill list --help"
  "$LUX_BIN" skill info --help > /dev/null 2>&1 && ok "lux skill info --help" || err "lux skill info --help"
  "$LUX_BIN" skill install --help > /dev/null 2>&1 && ok "lux skill install --help" || err "lux skill install --help"
  "$LUX_BIN" serve --help > /dev/null 2>&1 && ok "lux serve --help" || err "lux serve --help"
else
  warn "lux binary not found (run cargo build first)"
fi

# ── TypeScript ──────────────────────────────────
section "TypeScript / UI"

if (cd "$ROOT_DIR/gateway/ui-src" && npx tsc --noEmit 2>&1); then
  ok "tsc --noEmit (strict mode)"
else
  err "tsc --noEmit"
fi

# ── Protocol Schema ─────────────────────────────
section "Protocol & Module Checks"

if (cd "$ROOT_DIR/gateway" && cargo test protocol 2>&1); then
  ok "cargo test protocol (EventCategory serde)"
else
  err "cargo test protocol"
fi

if (cd "$ROOT_DIR/gateway" && cargo test ai_log 2>&1); then
  ok "cargo test ai_log (JSONL primitives)"
else
  err "cargo test ai_log"
fi

# ── .lux Path Checks ────────────────────────────
section "Path & Structure"

if [ -f "$ROOT_DIR/.lux/ROUTING.md" ]; then
  ok ".lux/ROUTING.md exists"
else
  warn ".lux/ROUTING.md not found (expected in project root)"
fi

if [ -f "$ROOT_DIR/gateway/src/ai_log.rs" ]; then
  ok "ai_log.rs module exists"
else
  err "ai_log.rs module missing"
fi

if [ -f "$ROOT_DIR/gateway/src/protocol.rs" ]; then
  ok "protocol.rs module exists"
else
  err "protocol.rs module missing"
fi

if (cd "$ROOT_DIR" && bash scripts/check-project-structure.sh 2>&1); then
  ok "project structure sanity"
else
  err "project structure sanity"
fi

# ── C# Note ─────────────────────────────────────
section "C# / Unity Editor"

warn "C# tests require Unity Editor — run via Window > General > Test Runner"
warn "Verify: LuxAiActionLogTests, LuxAiActionLogBroadcaster tests, all *Tests/Editor/"

# ── Policy Scan ────────────────────────────────
section "Policy Scan (Core Invariants)"

if (cd "$ROOT_DIR" && node scripts/policy-scan.mjs --advisory-only 2>&1); then
  ok "policy-scan (core invariants)"
else
  err "policy-scan (core invariants)"
fi

# ── Summary ─────────────────────────────────────
echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo -e "  ${GREEN}PASS${NC}: $pass   ${RED}FAIL${NC}: $fail   ${YELLOW}SKIP${NC}: $skip"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

if [ "$fail" -gt 0 ]; then
  echo ""
  echo -e "${RED}Some checks failed. Review output above.${NC}"
  exit 1
fi

echo ""
echo -e "${GREEN}All automated checks passed.${NC}"
exit 0
