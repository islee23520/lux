import * as fs from "node:fs"
import * as path from "node:path"
import type { LuxEvalResult, LuxPluginConfig, LuxSpecProject, PackageEntry, PackagesSpec } from "./types"

type UnityManifest = {
  dependencies?: Record<string, string>
}

const DOMAIN_NAMES = [
  "design",
  "architecture",
  "art_style",
  "audio",
  "narrative",
  "levels",
  "ui_ux",
] as const

// Read spec.json from project directory
export function loadSpec(projectPath: string): LuxSpecProject | null {
  const specPath = path.join(projectPath, ".lux", "spec.json")
  try {
    const content = fs.readFileSync(specPath, "utf-8")
    return JSON.parse(content) as LuxSpecProject
  } catch { /* intentional: missing or corrupt spec falls back to continuation-driven ambiguity */
    return null
  }
}

// Read Unity Packages/manifest.json to get detected packages
export function readUnityPackages(projectPath: string): PackageEntry[] {
  const manifestPath = path.join(projectPath, "Packages", "manifest.json")
  try {
    const content = fs.readFileSync(manifestPath, "utf-8")
    const manifest = JSON.parse(content) as UnityManifest
    return Object.entries(manifest.dependencies ?? {}).map(([name, version]) => ({
      name,
      reason: null,
      version,
    }))
  } catch { /* intentional: missing manifest means no detected packages */
    return []
  }
}

// Read Unity version from ProjectSettings
export function readUnityVersion(projectPath: string): string | null {
  const versionPath = path.join(projectPath, "ProjectSettings", "ProjectVersion.txt")
  try {
    const content = fs.readFileSync(versionPath, "utf-8")
    const match = content.match(/m_EditorVersion:\s*(.+)/)
    return match ? match[1]!.trim() : null
  } catch { /* intentional: missing version file means version is unknown */
    return null
  }
}

function hasAllRequiredPackages(packages: PackagesSpec, detectedPackages: PackageEntry[]): string[] {
  const detectedNames = new Set(detectedPackages.map((packageEntry) => packageEntry.name))
  return packages.required.filter((requiredPackage) => !detectedNames.has(requiredPackage.name)).map((packageEntry) => packageEntry.name)
}

// Evaluate spec vs actual project state
export function evaluateSpec(projectPath: string, config?: LuxPluginConfig): LuxEvalResult {
  const spec = loadSpec(projectPath)

  if (!spec) {
    // Missing spec is treated as intentionally ambiguous so ticket state still drives continuation.
    return {
      should_continue: true,
      next_action: "No spec.json found. Run 'lux spec init' or 'lux bridge install' first.",
      ambiguity_score: 1.0,
      continuation_count: 0,
    }
  }

  const issues: string[] = []
  let totalChecks = 0
  let passedChecks = 0

  // Check unity version
  totalChecks++
  if (spec.unity?.required_version) {
    const detected = readUnityVersion(projectPath)
    if (detected) {
      passedChecks++
    } else {
      issues.push("Unity version not detected. Specify required_version in spec.")
    }
  } else {
    issues.push("Unity version requirement not specified.")
  }

  // Check targets
  totalChecks++
  if (spec.targets && spec.targets.platforms.length > 0) {
    passedChecks++
  } else {
    issues.push("Target platforms not specified.")
  }

  // Check packages
  totalChecks++
  const detectedPackages = readUnityPackages(projectPath)
  if (spec.packages && spec.packages.required.length > 0) {
    const missingRequired = hasAllRequiredPackages(spec.packages, detectedPackages)
    if (missingRequired.length === 0) {
      passedChecks++
    } else {
      issues.push(`Missing required packages: ${missingRequired.join(", ")}`)
    }
  } else {
    issues.push("Required packages not specified.")
  }

  // Check testing
  totalChecks++
  if (spec.testing?.framework) {
    passedChecks++
  } else {
    issues.push("Test framework not specified.")
  }

  // Check domains
  for (const domain of DOMAIN_NAMES) {
    totalChecks++
    const domainSpec = spec.domains[domain]
    if (domainSpec && domainSpec.defined) {
      passedChecks++
    }
  }

  // Check glossary
  totalChecks++
  if (spec.glossary && spec.glossary.term_count > 0) {
    passedChecks++
  }

  // Ambiguity polarity: 0.0 = fully clear, 1.0 = maximally ambiguous
  const ambiguityScore = totalChecks > 0 ? 1 - passedChecks / totalChecks : 1.0
  const nextAction = issues.length > 0 ? issues[0]! : ""
  // Canonical ambiguity threshold matches lux_loop.rs DEFAULT_AMBIGUITY_THRESHOLD.
  const targetAmbiguity = config?.targetAmbiguity ?? 0.02
  const shouldContinueByThreshold = ambiguityScore > targetAmbiguity

  return {
    should_continue: config ? shouldContinueByThreshold : issues.length > 0,
    next_action: nextAction,
    ambiguity_score: Math.round(ambiguityScore * 100) / 100,
    continuation_count: 0,
  }
}

// Get next question based on spec evaluation (for Socratic dialogue)
export function getNextQuestion(projectPath: string): string | null {
  const result = evaluateSpec(projectPath)
  if (!result.should_continue) return null
  return result.next_action
}
