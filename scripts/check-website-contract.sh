#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SITE_DIR="$ROOT_DIR/website"

fail() {
  echo "$1" >&2
  exit 1
}

require_file() {
  local path="$1"
  [ -f "$ROOT_DIR/$path" ] || fail "missing website file: $path"
}

require_text() {
  local path="$1"
  local needle="$2"
  grep -Fq "$needle" "$ROOT_DIR/$path" || fail "$path must include: $needle"
}

require_file "website/index.html"
require_file "website/styles.css"
require_file "website/app.js"
require_file "website/.nojekyll"
require_file ".github/workflows/pages.yml"

require_text "website/index.html" '<html lang="en" data-lang="en">'
require_text "website/index.html" 'hreflang="en"'
require_text "website/index.html" 'hreflang="ko"'
require_text "website/index.html" 'data-lang-option="en"'
require_text "website/index.html" 'data-lang-option="ko"'
require_text "website/index.html" 'id="evidence-canvas"'
require_text "website/index.html" 'data-i18n="heroTitle"'
require_text "website/index.html" 'data-i18n="layerBridge"'
require_text "website/index.html" '../README.md'

require_text "website/app.js" 'const messages = {'
require_text "website/app.js" 'en: {'
require_text "website/app.js" 'ko: {'
require_text "website/app.js" 'function preferredLanguage()'
require_text "website/app.js" 'URLSearchParams'
require_text "website/app.js" 'setLanguage(preferredLanguage(), false)'
require_text "website/app.js" 'No git submodule or external bridge remote.'
require_text "website/app.js" 'runtime automation remains planned'
require_text "website/app.js" 'getContext("2d")'

require_text "website/styles.css" '#evidence-canvas'
require_text "website/styles.css" '.language-switcher'
require_text "website/styles.css" '@media (max-width: 620px)'
require_text "website/styles.css" 'prefers-reduced-motion'

require_text ".github/workflows/pages.yml" 'actions/configure-pages@v5'
require_text ".github/workflows/pages.yml" 'actions/upload-pages-artifact@v3'
require_text ".github/workflows/pages.yml" 'path: website'
require_text ".github/workflows/pages.yml" 'actions/deploy-pages@v4'
require_text ".github/workflows/pages.yml" 'workflow_dispatch'

if grep -RInE 'git submodule (update|init|add|sync)|initialize bridge submodule' "$SITE_DIR"; then
  fail "website must not instruct users to initialize bridge as a submodule"
fi

echo "website contract ok"
