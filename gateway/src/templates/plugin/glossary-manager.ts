declare module "node:fs" {
  export function readFileSync(filePath: string, encoding: "utf-8"): string
  export function writeFileSync(filePath: string, content: string, encoding: "utf-8"): void
  export function appendFileSync(filePath: string, content: string, encoding: "utf-8"): void
  export function existsSync(filePath: string): boolean
  export function mkdirSync(dirPath: string, options: { recursive: boolean }): void
}

declare module "node:path" {
  export function dirname(filePath: string): string
}

import * as fs from "node:fs"
import * as path from "node:path"
import type { LuxGlossaryEntry } from "./types"

const TABLE_HEADER = "| Term | Definition | Context | First Seen |"
const TABLE_SEPARATOR = "|------|-----------|---------|------------|"

export function loadGlossary(glossaryPath: string): LuxGlossaryEntry[] {
  const fullPath = glossaryPath

  try {
    const content = fs.readFileSync(fullPath, "utf-8")
    return parseGlossaryTable(content)
  } catch {
    return []
  }
}

export function parseGlossaryTable(content: string): LuxGlossaryEntry[] {
  const lines = content.split("\n")
  const entries: LuxGlossaryEntry[] = []

  for (const line of lines) {
    if (line.includes("Term") && line.includes("Definition")) {
      continue
    }

    if (/^\|[-|]+\|$/.test(line)) {
      continue
    }

    if (!line.startsWith("|")) {
      continue
    }

    const cells = line
      .split("|")
      .map((cell) => cell.trim())
      .filter((cell) => cell.length > 0)

    if (cells.length >= 4) {
      entries.push({
        term: cells[0],
        definition: cells[1],
        context: cells[2],
        first_seen: cells[3],
      })
    }
  }

  return entries
}

export function termExists(glossaryPath: string, term: string): boolean {
  const entries = loadGlossary(glossaryPath)
  return entries.some((entry) => entry.term.toLowerCase() === term.toLowerCase())
}

export function formatGlossaryEntry(entry: LuxGlossaryEntry): string {
  return `| ${entry.term} | ${entry.definition} | ${entry.context} | ${entry.first_seen} |`
}

export function appendTerm(glossaryPath: string, entry: LuxGlossaryEntry): boolean {
  if (termExists(glossaryPath, entry.term)) {
    return false
  }

  const fullPath = glossaryPath
  const dir = path.dirname(fullPath)

  if (!fs.existsSync(dir)) {
    fs.mkdirSync(dir, { recursive: true })
  }

  if (!fs.existsSync(fullPath)) {
    const header = `# Project Glossary\n\n> Auto-managed by Lux. Terms discovered during development are appended here.\n\n## Terms\n\n${TABLE_HEADER}\n${TABLE_SEPARATOR}\n`
    fs.writeFileSync(fullPath, header, "utf-8")
  }

  const row = formatGlossaryEntry(entry)
  fs.appendFileSync(fullPath, `${row}\n`, "utf-8")

  return true
}

export function updateGlossarySpec(specPath: string, glossaryPath: string): void {
  const entries = loadGlossary(glossaryPath)

  try {
    const content = fs.readFileSync(specPath, "utf-8")
    const spec = JSON.parse(content) as Record<string, unknown>

    if (!spec.glossary || typeof spec.glossary !== "object") {
      spec.glossary = {}
    }

    const glossary = spec.glossary as Record<string, unknown>
    glossary.term_count = entries.length
    glossary.last_updated = new Date().toISOString()

    fs.writeFileSync(specPath, `${JSON.stringify(spec, null, 2)}\n`, "utf-8")
  } catch (error) {
    console.warn("[lux-glossary-manager] Failed to update spec glossary:", error)
  }
}

export function getAllTerms(glossaryPath: string): string[] {
  return loadGlossary(glossaryPath).map((entry) => entry.term)
}
