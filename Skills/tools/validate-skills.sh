#!/usr/bin/env bash

set -u

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
: "${SKILLS_ROOT:=$REPO_ROOT/skills}"

overall_status=0

print_result() {
  local status="$1"
  local label="$2"
  local details="$3"
  printf '%s: %s - %s\n' "$status" "$label" "$details"
}

check_frontmatter_field() {
  local file="$1"
  local field="$2"
  awk -v target="$field" '
    NR == 1 && $0 != "---" { exit 2 }
    NR > 1 {
      if ($0 == "---") { exit found ? 0 : 1 }
      if ($0 ~ "^" target ":[[:space:]]+") { found = 1 }
    }
    END {
      if (NR == 0) { exit 2 }
      if (!found) { exit 1 }
    }
  ' "$file"
}

if [ ! -d "$SKILLS_ROOT" ]; then
  print_result "FAIL" "skills-root" "$SKILLS_ROOT does not exist"
  exit 1
fi

required_categories=(
  architecture
  review
  workflow
  unity
  studio
  quality
  bugs
)

for category in "${required_categories[@]}"; do
  category_dir="$SKILLS_ROOT/$category"
  if [ ! -d "$category_dir" ]; then
    print_result "FAIL" "category-$category" "missing category directory"
    overall_status=1
    continue
  fi
  if [ ! -f "$category_dir/AGENTS.md" ]; then
    print_result "FAIL" "category-$category" "missing AGENTS.md routing file"
    overall_status=1
  else
    print_result "PASS" "category-$category" "AGENTS.md routing file exists"
  fi
done

flat_skill_dirs="$({
  find "$SKILLS_ROOT" -mindepth 1 -maxdepth 1 -type d \
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
  print_result "FAIL" "category-layout" "skills must live below category directories"
  printf '%s\n' "$flat_skill_dirs"
  overall_status=1
else
  print_result "PASS" "category-layout" "no flat skill directories"
fi

skill_count=0
while IFS= read -r -d '' skill_dir; do
  skill_count=$((skill_count + 1))
  skill_name="$(basename "$skill_dir")"
  skill_file="$skill_dir/SKILL.md"
  relative_skill_dir="${skill_dir#"$SKILLS_ROOT"/}"
  category_name="${relative_skill_dir%%/*}"

  if [ ! -f "$skill_file" ]; then
    print_result "FAIL" "$skill_name" "missing SKILL.md"
    overall_status=1
    continue
  fi

  if [ -L "$skill_file" ]; then
    link_target="$(readlink "$skill_file")"
    case "$link_target" in
      /*)
        print_result "FAIL" "$skill_name" "SKILL.md must not be an absolute symlink to $link_target"
        overall_status=1
        continue
        ;;
    esac
  fi

  print_result "PASS" "$skill_name" "SKILL.md exists"

  if awk 'NR == 1 { exit $0 == "---" ? 0 : 1 }' "$skill_file"; then
    if awk 'NR > 1 && $0 == "---" { found = 1; exit 0 } END { exit found ? 0 : 1 }' "$skill_file"; then
      print_result "PASS" "$skill_name" "valid frontmatter delimiters"
    else
      print_result "FAIL" "$skill_name" "missing closing frontmatter delimiter"
      overall_status=1
    fi
  else
    print_result "FAIL" "$skill_name" "missing opening frontmatter delimiter"
    overall_status=1
  fi

  for field in name description category source; do
    if check_frontmatter_field "$skill_file" "$field"; then
      print_result "PASS" "$skill_name" "frontmatter contains $field"
    else
      print_result "FAIL" "$skill_name" "frontmatter missing $field"
      overall_status=1
    fi
  done

  declared_category="$({
    awk '/^category:[[:space:]]+/ { sub(/^category:[[:space:]]+/, "", $0); print; exit }' "$skill_file"
  })"
  if [ "$declared_category" = "$category_name" ]; then
    print_result "PASS" "$skill_name" "frontmatter category matches $category_name"
  else
    print_result "FAIL" "$skill_name" "frontmatter category '$declared_category' does not match category '$category_name'"
    overall_status=1
  fi

  manifest_file="$skill_dir/manifest.json"
  if [ -f "$manifest_file" ]; then
    manifest_category="$(
      node -e 'const fs=require("fs"); const value=JSON.parse(fs.readFileSync(process.argv[1],"utf8")); process.stdout.write(value.category || "");' "$manifest_file"
    )"
    if [ "$manifest_category" = "$category_name" ]; then
      print_result "PASS" "$skill_name" "manifest category matches $category_name"
    else
      print_result "FAIL" "$skill_name" "manifest category '$manifest_category' does not match category '$category_name'"
      overall_status=1
    fi
  fi

  declared_name="$({
    awk '/^name:[[:space:]]+/ { sub(/^name:[[:space:]]+/, "", $0); print; exit }' "$skill_file"
  })"
  if [ "$declared_name" = "$skill_name" ]; then
    print_result "PASS" "$skill_name" "name matches directory"
  else
    print_result "FAIL" "$skill_name" "name '$declared_name' does not match directory '$skill_name'"
    overall_status=1
  fi

  line_count="$(wc -l < "$skill_file" | tr -d ' ')"
  if [ "$line_count" -le 500 ]; then
    print_result "PASS" "$skill_name" "line count $line_count within limit"
  else
    print_result "FAIL" "$skill_name" "line count $line_count exceeds 500"
    overall_status=1
  fi
done < <(find "$SKILLS_ROOT" -mindepth 2 -maxdepth 4 -type d -exec test -f '{}/SKILL.md' ';' -print0 | sort -z)

if [ "$skill_count" -eq 0 ]; then
  print_result "FAIL" "skills" "no SKILL.md files found below category directories"
  overall_status=1
else
  print_result "PASS" "skills" "validated $skill_count skill directories"
fi

if [ "$overall_status" -eq 0 ]; then
  print_result "PASS" "summary" "all skill checks passed"
else
  print_result "FAIL" "summary" "one or more skill checks failed"
fi

exit "$overall_status"
