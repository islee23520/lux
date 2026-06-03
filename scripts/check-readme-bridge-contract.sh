#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"

fail() {
  echo "$1" >&2
  exit 1
}

require_readme_text() {
  local needle="$1"
  if ! grep -Fq "$needle" "$ROOT_DIR/README.md"; then
    fail "README.md must include: $needle"
  fi
}

require_korean_readme_text() {
  local needle="$1"
  if ! grep -Fq "$needle" "$ROOT_DIR/README.ko.md"; then
    fail "README.ko.md must include: $needle"
  fi
}

if [ -f "$ROOT_DIR/.gitmodules" ] && grep -Eq '(^\[submodule "bridge"\]|path = bridge|lux-bridge)' "$ROOT_DIR/.gitmodules"; then
  fail "bridge must not be registered in .gitmodules"
fi

if git -C "$ROOT_DIR" ls-files --stage bridge | awk '$1 == "160000" { found = 1 } END { exit found ? 0 : 1 }'; then
  fail "bridge must be regular tracked files, not a gitlink"
fi

tracked_bridge_sources="$(git -C "$ROOT_DIR" ls-files 'bridge/*')"
for path in \
  bridge/unity/AiBridgeEditor/UnityAiBridge.cs \
  bridge/unity/AiBridgeEditor/UnityAiBridge.cs.meta \
  bridge/unity/package.json \
  bridge/unity/package.json.meta \
  bridge/godot/bridge.gd \
  bridge/threejs/dist/src/server.js
do
  if ! printf '%s\n' "$tracked_bridge_sources" | grep -Fxq "$path"; then
    fail "bridge source must be tracked as an in-repo file: $path"
  fi
done

if git -C "$ROOT_DIR" ls-files 'bridge/**/node_modules/**' | grep -q .; then
  fail "bridge dependency directories must not be tracked"
fi

if grep -Eiq 'git submodule (update|init|add|sync)|initialize bridge submodule' "$ROOT_DIR/README.md"; then
  fail "README.md must not instruct users to initialize bridge as a submodule"
fi

if [ ! -f "$ROOT_DIR/README.ko.md" ]; then
  fail "README.ko.md must exist as the Korean translation"
fi

if grep -Eq '[가-힣]' "$ROOT_DIR/README.md"; then
  fail "README.md must stay English-only; put Korean content in README.ko.md"
fi

if grep -Eiq 'git submodule (update|init|add|sync)|initialize bridge submodule|브릿지 서브모듈 초기화' "$ROOT_DIR/README.ko.md"; then
  fail "README.ko.md must not instruct users to initialize bridge as a submodule"
fi

require_readme_text "[Korean](README.ko.md)"
require_readme_text "This English README is the base version."
require_readme_text "## LUX Rhythm"
require_readme_text "## Content Areas"
require_readme_text '| `.lux/` | Runtime truth'
require_readme_text '| `gateway/` | Control-plane runtime'
require_readme_text '| `bridge/` | In-repository engine bridge source'
require_readme_text '| `Skills/` | Agent workflow library'
require_readme_text "Bridge sources are ordinary files in this repository"
require_readme_text "Three.js remains planned unless a runtime harness is present and verified"

require_korean_readme_text "[English](README.md)"
require_korean_readme_text "영어 README가 기준 문서입니다."
require_korean_readme_text "## LUX 리듬"
require_korean_readme_text "## 콘텐츠 영역"
require_korean_readme_text '| `.lux/` | 런타임 진실'
require_korean_readme_text '| `bridge/` | 저장소 내부 엔진 브릿지 소스'

echo "README and bridge contract ok"
