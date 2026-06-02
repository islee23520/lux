import { describe, it, expect, vi, beforeEach } from "vitest"
import * as fs from "node:fs"
import * as path from "node:path"
import { loadSpec, readUnityPackages, readUnityVersion, evaluateSpec, getNextQuestion } from "../spec-evaluator"
import type { LuxSpecProject, LuxPluginConfig } from "../types"

vi.mock("node:fs")

describe("spec-evaluator", () => {
  const projectPath = "/test/project"

  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(fs.existsSync).mockReturnValue(true)
  })

  describe("loadSpec", () => {
    it("should load and parse canonical spec successfully", () => {
      const mockSpec: Partial<LuxSpecProject> = { project_name: "Test Project" }
      vi.mocked(fs.existsSync).mockImplementation((p) =>
        p.toString() === path.join(projectPath, ".lux", "specs", "spec.json")
      )
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(mockSpec))

      const result = loadSpec(projectPath)
      expect(result).toEqual(mockSpec)
      expect(fs.readFileSync).toHaveBeenCalledWith(
        path.join(projectPath, ".lux", "specs", "spec.json"),
        "utf-8"
      )
    })

    it("should use legacy compatibility fallback when canonical spec is missing", () => {
      const mockSpec: Partial<LuxSpecProject> = { project_name: "Legacy Project" }
      vi.mocked(fs.existsSync).mockImplementation((p) =>
        p.toString() === path.join(projectPath, ".lux", "spec.json")
      )
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(mockSpec))

      const result = loadSpec(projectPath)
      expect(result).toEqual(mockSpec)
      expect(fs.readFileSync).toHaveBeenCalledWith(
        path.join(projectPath, ".lux", "spec.json"),
        "utf-8"
      )
    })

    it("should return null if spec.json is missing", () => {
      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = loadSpec(projectPath)
      expect(result).toBeNull()
      expect(fs.readFileSync).not.toHaveBeenCalled()
    })

    it("should return null if spec.json is invalid JSON", () => {
      vi.mocked(fs.existsSync).mockReturnValue(true)
      vi.mocked(fs.readFileSync).mockReturnValue("invalid json")

      const result = loadSpec(projectPath)
      expect(result).toBeNull()
    })
  })

  describe("readUnityPackages", () => {
    it("should read packages from manifest.json", () => {
      const mockManifest = {
        dependencies: {
          "com.unity.render-pipelines.universal": "14.0.11",
          "com.unity.test-framework": "1.1.33"
        }
      }
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify(mockManifest))

      const result = readUnityPackages(projectPath)
      expect(result).toHaveLength(2)
      expect(result).toContainEqual({ name: "com.unity.render-pipelines.universal", reason: null, version: "14.0.11" })
      expect(result).toContainEqual({ name: "com.unity.test-framework", reason: null, version: "1.1.33" })
    })

    it("should return empty array if manifest.json is missing", () => {
      vi.mocked(fs.readFileSync).mockImplementation(() => {
        throw new Error("File not found")
      })

      const result = readUnityPackages(projectPath)
      expect(result).toEqual([])
    })
  })

  describe("readUnityVersion", () => {
    it("should extract version from ProjectVersion.txt", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("m_EditorVersion: 6000.0.0f1\nm_EditorVersionWithRevision: 6000.0.0f1 (abc)")

      const result = readUnityVersion(projectPath)
      expect(result).toBe("6000.0.0f1")
    })

    it("should return null if version file is missing", () => {
      vi.mocked(fs.readFileSync).mockImplementation(() => {
        throw new Error("File not found")
      })

      const result = readUnityVersion(projectPath)
      expect(result).toBeNull()
    })

    it("should return null if version pattern is not found", () => {
      vi.mocked(fs.readFileSync).mockReturnValue("some other content")

      const result = readUnityVersion(projectPath)
      expect(result).toBeNull()
    })

    it("should read packages with empty dependencies object", () => {
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ dependencies: {} }))

      const result = readUnityPackages(projectPath)
      expect(result).toEqual([])
    })

  })

  describe("evaluateSpec", () => {
    const createMockSpec = (overrides: any = {}): LuxSpecProject => ({
      version: "1.0.0",
      project_id: "test-id",
      project_name: "Test Project",
      created_at: "2024-01-01",
      updated_at: "2024-01-01",
      source: "manual",
      status: "Active",
      domains: {
        design: { defined: true } as any,
        architecture: { defined: true } as any,
        art_style: { defined: true } as any,
        audio: { defined: true } as any,
        narrative: { defined: true } as any,
        levels: { defined: true } as any,
        ui_ux: { defined: true } as any,
        custom: {}
      },
      schell_evaluation: {} as any,
      overall_ambiguity: 0,
      unity: { required_version: "6000.0.0f1" } as any,
      targets: { platforms: ["macOS"] } as any,
      packages: { required: [{ name: "com.unity.test-framework" }] } as any,
      testing: { framework: "Unity Test Framework" } as any,
      glossary: { term_count: 5 } as any,
      ...overrides
    })

    it("should return default result if spec is missing", () => {
      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = evaluateSpec(projectPath)
      expect(result.should_continue).toBe(true)
      expect(result.ambiguity_score).toBe(1.0)
      expect(result.next_action).toContain("No spec.json found")
    })

    it("should calculate high ambiguity score when spec is incomplete", () => {
      const incompleteSpec = createMockSpec({
        unity: null,
        targets: null,
        packages: null,
        testing: null,
        domains: {
          design: { defined: false },
          architecture: { defined: false },
          art_style: { defined: false },
          audio: { defined: false },
          narrative: { defined: false },
          levels: { defined: false },
          ui_ux: { defined: false },
          custom: {}
        },
        glossary: { term_count: 0 }
      })

      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        if (p.toString().includes("spec.json")) return JSON.stringify(incompleteSpec)
        throw new Error("File not found")
      })

      const result = evaluateSpec(projectPath)
      expect(result.ambiguity_score).toBeGreaterThan(0.5)
      expect(result.should_continue).toBe(true)
    })

    it("should calculate low ambiguity score when spec is complete", () => {
      const completeSpec = createMockSpec()
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        const pathStr = p.toString()
        if (pathStr.includes("spec.json")) return JSON.stringify(completeSpec)
        if (pathStr.includes("manifest.json")) return JSON.stringify({ dependencies: { "com.unity.test-framework": "1.0.0" } })
        if (pathStr.includes("ProjectVersion.txt")) return "m_EditorVersion: 6000.0.0f1"
        return ""
      })

      const result = evaluateSpec(projectPath)
      expect(result.ambiguity_score).toBe(0)
      expect(result.should_continue).toBe(false)
    })

    it("should follow the ambiguity polarity and convergence contract", () => {
      const incompleteSpec = createMockSpec({
        unity: null,
        targets: null,
        packages: null,
        testing: null,
        domains: {
          design: { defined: false },
          architecture: { defined: false },
          art_style: { defined: false },
          audio: { defined: false },
          narrative: { defined: false },
          levels: { defined: false },
          ui_ux: { defined: false },
          custom: {}
        },
        glossary: { term_count: 0 }
      })
      const completeSpec = createMockSpec()

      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        const pathStr = p.toString()
        if (pathStr.includes("spec.json")) return JSON.stringify(incompleteSpec)
        throw new Error("File not found")
      })
      const incomplete = evaluateSpec(projectPath, { targetAmbiguity: 0.02 } as LuxPluginConfig)
      expect(incomplete.ambiguity_score).toBeGreaterThan(0.5)
      expect(incomplete.ambiguity_score).toBeGreaterThan(0.02)
      expect(incomplete.should_continue).toBe(true)

      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        const pathStr = p.toString()
        if (pathStr.includes("spec.json")) return JSON.stringify(completeSpec)
        if (pathStr.includes("manifest.json")) return JSON.stringify({ dependencies: { "com.unity.test-framework": "1.0.0" } })
        if (pathStr.includes("ProjectVersion.txt")) return "m_EditorVersion: 6000.0.0f1"
        return ""
      })
      const complete = evaluateSpec(projectPath, { targetAmbiguity: 0.02 } as LuxPluginConfig)
      expect(complete.ambiguity_score).toBeLessThanOrEqual(0.02)
      expect(complete.should_continue).toBe(false)
    })

    it("should surface the first issue as the next action", () => {
      const spec = createMockSpec({ unity: null })
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        const pathStr = p.toString()
        if (pathStr.includes("spec.json")) return JSON.stringify(spec)
        return ""
      })

      const result = evaluateSpec(projectPath)
      expect(result.next_action).toContain("Unity version requirement not specified")
    })

    it("should respect targetAmbiguity threshold from config", () => {
      const spec = createMockSpec({
        unity: null
      })
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        if (p.toString().includes("spec.json")) return JSON.stringify(spec)
        return ""
      })

      const config: LuxPluginConfig = {
        maxContinuations: 10,
        specPath: ".lux/spec.json",
        glossaryPath: ".lux/glossary.md",
        targetAmbiguity: 0.01
      }

      const result = evaluateSpec(projectPath, config)
      expect(result.should_continue).toBe(true)

      const looseConfig: LuxPluginConfig = {
        ...config,
        targetAmbiguity: 0.9
      }
      const resultLoose = evaluateSpec(projectPath, looseConfig)
      expect(resultLoose.should_continue).toBe(false)
    })

  })

  describe("getNextQuestion", () => {
    it("should return next action if evaluation says should continue", () => {
      vi.mocked(fs.existsSync).mockReturnValue(false)

      const result = getNextQuestion(projectPath)
      expect(result).toContain("No spec.json found")
    })

    it("should return null if evaluation says should not continue", () => {
      const completeSpec = {
        domains: {
          design: { defined: true },
          architecture: { defined: true },
          art_style: { defined: true },
          audio: { defined: true },
          narrative: { defined: true },
          levels: { defined: true },
          ui_ux: { defined: true }
        },
        unity: { required_version: "6000.0.0f1" },
        targets: { platforms: ["macOS"] },
        packages: { required: [{ name: "com.unity.test-framework" }] },
        testing: { framework: "UTF" },
        glossary: { term_count: 1 }
      }
      vi.mocked(fs.readFileSync).mockImplementation((p) => {
        const pathStr = p.toString()
        if (pathStr.includes("spec.json")) return JSON.stringify(completeSpec)
        if (pathStr.includes("ProjectVersion.txt")) return "m_EditorVersion: 6000.0.0f1"
        if (pathStr.includes("manifest.json")) return JSON.stringify({ dependencies: { "com.unity.test-framework": "1.0.0" } })
        return ""
      })

      const result = getNextQuestion(projectPath)
      expect(result).toBeNull()
    })
  })
})
