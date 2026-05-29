#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

required_roots=(
  "gateway"
  "bridge"
  "adapters"
  "Skills"
  "Skills/skills"
  "docs"
  "scripts"
)

for path in "${required_roots[@]}"; do
  if [ ! -d "$ROOT_DIR/$path" ]; then
    echo "missing required feature root: $path" >&2
    exit 1
  fi
done

removed_roots=(
  "seeds"
  "plugins"
  "bridge-threejs"
)

for path in "${removed_roots[@]}"; do
  if [ -e "$ROOT_DIR/$path" ]; then
    echo "removed legacy root must not be active source: $path" >&2
    exit 1
  fi
done

tracked_forbidden="$({
  cd "$ROOT_DIR"
  git ls-files | grep -E '(^|/)(node_modules|target|ui_smoke_test_renamed)(/|$)' | while IFS= read -r path; do
    [ -e "$path" ] && printf '%s\n' "$path"
  done
} || true)"
if [ -n "$tracked_forbidden" ]; then
  echo "tracked generated or test-temporary paths must not be part of the source hierarchy:" >&2
  echo "$tracked_forbidden" >&2
  exit 1
fi

if [ ! -f "$ROOT_DIR/gateway/Cargo.toml" ]; then
  echo "gateway/Cargo.toml missing; gateway must remain the Rust CLI/server root" >&2
  exit 1
fi

if ! find "$ROOT_DIR/Skills/skills" -mindepth 2 -maxdepth 2 -name manifest.json | grep -q .; then
  echo "Skills must contain per-skill manifest.json files" >&2
  exit 1
fi

if [ ! -f "$ROOT_DIR/adapters/opencode/lux-plugin.ts" ]; then
  echo "adapters/opencode/lux-plugin.ts missing; OpenCode adapter source must remain tracked" >&2
  exit 1
fi

echo "project structure sanity ok"
