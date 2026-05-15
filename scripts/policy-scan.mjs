#!/usr/bin/env node
// policy-scan.mjs — Scan LUX source for core invariant violations.
// Usage: node scripts/policy-scan.mjs [--advisory-only]

import { readFileSync, readdirSync } from 'node:fs';
import { join, extname, relative } from 'node:path';

const ROOT = new URL('..', import.meta.url).pathname;
const ADVISORY_ONLY = process.argv.includes('--advisory-only');

const SCAN_EXTENSIONS = new Set(['.rs', '.ts', '.tsx', '.cs']);
const SKIP_DIRS = new Set(['node_modules', 'target', 'dist', '.lux', 'Skills', '.git', 'references']);

const ALLOW_MARKERS = ['lux-allow-failover', 'lux-allow-legacy', 'lux-allow-dual-write'];

// Heuristic rules: [regex, invariant, severity, description]
const RULES = [
  // Silent Fallback — empty catch blocks
  [/catch\s*\([^)]*\)\s*\{\s*\}/, 'No Silent Fallback', 'warning', 'Empty catch block — error is silently swallowed'],
  // Silent Fallback — Rust unwrap_or_default without marker
  [/unwrap_or_default\(\)/, 'No Silent Fallback', 'advisory', 'unwrap_or_default() silently provides a default value'],
  // Silent Fallback — Rust ok() without handling
  [/\.ok\(\)\s*[;\n]/, 'No Silent Fallback', 'advisory', '.ok() discards the error without logging'],
  // Legacy / shadow paths
  [/\b(legacy|deprecated|compat[_-]?path|old[_-]?path|shadow[_-]?path|workaround)\b/i, 'SSoT', 'advisory', 'Potential legacy or shadow path detected'],
  // Dual-write patterns
  [/\b(write\s*.*\s+both|mirror[_-]?write|fan[_-]?out\s+write)\b/i, 'SSoT', 'warning', 'Potential dual-write pattern detected'],
  // Non-idempotent — side effects in GET handler (Rust axum)
  [/\.(get|head)\s*\([^)]*\).*\b(mut\s|insert|push|write|create|delete|remove)\b/s, 'Idempotency', 'warning', 'GET/HEAD handler appears to have side effects'],
  // Non-idempotent — side effects in GET handler (TS/Express-style)
  [/app\.(get|head)\s*\([^)]*\).*\b(create|update|delete|insert|write)\b/s, 'Idempotency', 'warning', 'GET/HEAD handler appears to have side effects'],
  // TODO/FIXME/HACK comments
  [/\b(TODO|FIXME|HACK)\b/, 'Consistency', 'advisory', 'TODO/FIXME/HACK comment found — resolve before merge'],
];

let findings = [];

function hasAllowMarker(lines, lineIdx) {
  const contextRange = 3;
  for (let i = Math.max(0, lineIdx - contextRange); i <= Math.min(lines.length - 1, lineIdx + contextRange); i++) {
    for (const marker of ALLOW_MARKERS) {
      if (lines[i].includes(marker)) return true;
    }
  }
  return false;
}

function scanFile(filePath) {
  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');
  const rel = relative(ROOT, filePath);

  for (const [regex, invariant, severity, description] of RULES) {
    const matches = content.matchAll(new RegExp(regex.source, regex.flags.includes('g') ? regex.flags : `${regex.flags}g`));
    for (const match of matches) {
      // Find line number
      const before = content.slice(0, match.index);
      const lineNum = (before.match(/\n/g) || []).length + 1;
      
      if (hasAllowMarker(lines, lineNum - 1)) continue;
      
      findings.push({ severity, invariant, file: rel, line: lineNum, description });
    }
  }
}

function walk(dir) {
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    if (entry.isDirectory()) {
      if (!SKIP_DIRS.has(entry.name)) walk(join(dir, entry.name));
    } else if (SCAN_EXTENSIONS.has(extname(entry.name))) {
      scanFile(join(dir, entry.name));
    }
  }
}

// ── Main ──────────────────────────────────

walk(ROOT);

// Sort: critical > warning > advisory
const severityOrder = { critical: 0, warning: 1, advisory: 2 };
findings.sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);

// Output
const criticals = findings.filter(f => f.severity === 'critical');
const warnings = findings.filter(f => f.severity === 'warning');
const advisories = findings.filter(f => f.severity === 'advisory');

for (const f of findings) {
  const tag = f.severity.toUpperCase().padEnd(9);
  console.log(`[${tag}] ${f.file}:${f.line} — [${f.invariant}] ${f.description}`);
}

console.log('');
console.log(`  Critical: ${criticals.length}  Warning: ${warnings.length}  Advisory: ${advisories.length}  Total: ${findings.length}`);

if (ADVISORY_ONLY) {
  console.log('  (--advisory-only: exit 0 regardless)');
  process.exit(0);
}

if (criticals.length > 0 || warnings.length > 0) {
  console.log('  Policy check found critical or warning violations.');
  process.exit(1);
}

console.log('  Policy check passed (advisory-only findings may exist).');
process.exit(0);
