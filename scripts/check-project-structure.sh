#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

required_roots=(
  "gateway"
  "crates"
  "bridge"
  "Skills"
  "Skills/skills"
  "Skills/skills/architecture"
  "Skills/skills/review"
  "Skills/skills/workflow"
  "Skills/skills/unity"
  "Skills/skills/studio"
  "Skills/skills/quality"
  "Skills/skills/bugs"
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
  "adapters"
  "seeds"
  "plugins"
  "bridge-threejs"
  "gateway/ui"
  "gateway/ui-src"
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

flat_skill_dirs="$({
  find "$ROOT_DIR/Skills/skills" -mindepth 1 -maxdepth 1 -type d \
    ! -name architecture \
    ! -name review \
    ! -name workflow \
    ! -name unity \
    ! -name studio \
    ! -name quality \
    ! -name bugs \
    -print
} || true)"
if [ -n "$flat_skill_dirs" ]; then
  echo "Skills must live under category directories, not as flat Skills/skills children:" >&2
  echo "$flat_skill_dirs" >&2
  exit 1
fi

if ! find "$ROOT_DIR/Skills/skills" -mindepth 3 -maxdepth 5 -name manifest.json | grep -q .; then
  echo "Skills must contain per-skill manifest.json files" >&2
  exit 1
fi

echo "project structure sanity ok"
