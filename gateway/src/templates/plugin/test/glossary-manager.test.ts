import { describe, it, expect, vi, beforeEach } from "vitest"
import * as fs from "node:fs"
import * as path from "node:path"
import {
  loadGlossary,
  parseGlossaryTable,
  termExists,
  formatGlossaryEntry,
  appendTerm,
  updateGlossarySpec,
  getAllTerms,
} from "../glossary-manager"

vi.mock("node:fs")
vi.mock("node:path")

describe("glossary-manager", () => {
  const mockGlossaryPath = "/mock/project/.lux/glossary.md"
  const mockSpecPath = "/mock/project/.lux/spec.json"


  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(path.dirname).mockImplementation((p) => p.substring(0, p.lastIndexOf("/")))
  })

  describe("parseGlossaryTable", () => {
    it("should parse a valid markdown table", () => {
      const content = `
| Term | Definition | Context | First Seen |
|------|-----------|---------|------------|
| Unity | Game Engine | Core | 2026-05-13 |
| Rust | Systems Lang | Gateway | 2026-05-13 |
`
      const entries = parseGlossaryTable(content)
      expect(entries).toHaveLength(2)
      expect(entries[0]).toEqual({
        term: "Unity",
        definition: "Game Engine",
        context: "Core",
        first_seen: "2026-05-13",
      })
    })

    it("should skip header and separator lines", () => {
      const content = `| Term | Definition | Context | First Seen |\n|------|-----------|---------|------------|`
      const entries = parseGlossaryTable(content)
      expect(entries).toHaveLength(0)
    })

    it("should skip lines that do not start with |", () => {
      const content = "Some random text\n| Term | Def | Ctx | Date |"
      const entries = parseGlossaryTable(content)
      expect(entries).toHaveLength(1)
      expect(entries[0].term).toBe("Term")
    })
  })

  describe("loadGlossary", () => {
    it("should return empty array if file does not exist", () => {
      vi.mocked(fs.readFileSync).mockImplementation(() => {
        throw new Error("File not found")
      })
      const entries = loadGlossary(mockGlossaryPath)
      expect(entries).toEqual([])
    })

    it("should return parsed entries if file exists", () => {
      const content = "| Term | Definition | Context | First Seen |\n|------|-----------|---------|------------|\n| Lux | Toolkit | Project | 2026 |"
      vi.mocked(fs.readFileSync).mockReturnValue(content)
      const entries = loadGlossary(mockGlossaryPath)
      expect(entries).toHaveLength(1)
      expect(entries[0].term).toBe("Lux")
    })
  })

  describe("termExists", () => {
    it("should return true if term exists (case-insensitive)", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| T1 | D1 | C1 | S1 |")
      expect(termExists(mockGlossaryPath, "t1")).toBe(true)
      expect(termExists(mockGlossaryPath, "T1")).toBe(true)
    })

    it("should return false if term does not exist", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| T1 | D1 | C1 | S1 |")
      expect(termExists(mockGlossaryPath, "T2")).toBe(false)
    })
  })

  describe("formatGlossaryEntry", () => {
    it("should format entry as markdown table row", () => {
      const entry = { term: "A", definition: "B", context: "C", first_seen: "D" }
      expect(formatGlossaryEntry(entry)).toBe("| A | B | C | D |")
    })
  })

  describe("appendTerm", () => {
    it("should return false if term already exists", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| Existing | D | C | S |")
      const result = appendTerm(mockGlossaryPath, { term: "Existing", definition: "", context: "", first_seen: "" })
      expect(result).toBe(false)
      expect(fs.appendFileSync).not.toHaveBeenCalled()
    })

    it("should create file with header if it does not exist", () => {
      vi.mocked(fs.readFileSync).mockImplementation(() => { throw new Error() })
      vi.mocked(fs.existsSync).mockReturnValue(false) 

      appendTerm(mockGlossaryPath, { term: "New", definition: "Def", context: "Ctx", first_seen: "Now" })

      expect(fs.mkdirSync).toHaveBeenCalledWith(path.dirname(mockGlossaryPath), { recursive: true })
      expect(fs.writeFileSync).toHaveBeenCalledWith(mockGlossaryPath, expect.stringContaining("# Project Glossary"), "utf-8")
      expect(fs.appendFileSync).toHaveBeenCalledWith(mockGlossaryPath, "| New | Def | Ctx | Now |\n", "utf-8")
    })

    it("should append to existing file", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| Old | D | C | S |")
      vi.mocked(fs.existsSync).mockReturnValue(true)

      appendTerm(mockGlossaryPath, { term: "New", definition: "Def", context: "Ctx", first_seen: "Now" })

      expect(fs.writeFileSync).not.toHaveBeenCalled()
      expect(fs.appendFileSync).toHaveBeenCalledWith(mockGlossaryPath, "| New | Def | Ctx | Now |\n", "utf-8")
    })

    it("should skip duplicate term and keep existing content unchanged", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| Old | D | C | S |")
      vi.mocked(fs.existsSync).mockReturnValue(true)

      const result = appendTerm(mockGlossaryPath, { term: "Old", definition: "Def", context: "Ctx", first_seen: "Now" })

      expect(result).toBe(false)
      expect(fs.writeFileSync).not.toHaveBeenCalled()
    })
  })

  describe("updateGlossarySpec", () => {
    it("should update spec file with term count and timestamp", () => {
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        if (p === mockGlossaryPath) return "| T1 | D1 | C1 | S1 |"
        if (p === mockSpecPath) return JSON.stringify({ name: "test" })
        throw new Error()
      })

      updateGlossarySpec(mockSpecPath, mockGlossaryPath)

      expect(fs.writeFileSync).toHaveBeenCalledWith(
        mockSpecPath,
        expect.stringContaining('"term_count": 1'),
        "utf-8"
      )
    })

    it("should handle missing glossary object in spec", () => {
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        if (p === mockGlossaryPath) return ""
        if (p === mockSpecPath) return "{}"
        throw new Error()
      })

      updateGlossarySpec(mockSpecPath, mockGlossaryPath)
      expect(fs.writeFileSync).toHaveBeenCalled()
    })
  })

  describe("getAllTerms", () => {
    it("should return all terms from glossary", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("| T1 | D1 | C1 | S1 |\n| T2 | D2 | C2 | S2 |")
      expect(getAllTerms(mockGlossaryPath)).toEqual(["T1", "T2"])
    })
  })
})
