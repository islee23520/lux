import { describe, it, expect, vi, beforeEach } from "vitest"
import * as fs from "node:fs"
import * as path from "node:path"
import { createExternalSignalIntegrator, readExecutionLogRecords } from "../external-signal-integrator"

vi.mock("node:fs")
vi.mock("node:path")

describe("external-signal-integrator", () => {
  const mockProjectPath = "/mock/project"
  const mockLogPath = "/mock/project/.lux/execution-log.jsonl"

  beforeEach(() => {
    vi.resetAllMocks()
    vi.mocked(path.join).mockImplementation((...args) => args.join("/"))
  })

  it("should initialize with empty history", () => {
    const integrator = createExternalSignalIntegrator()
    expect(integrator.getRecentResults()).toEqual([])
    expect(integrator.getHealthScore()).toBe(100)
  })

  describe("reporting results", () => {
    it("should report build result and persist it", () => {
      const integrator = createExternalSignalIntegrator(mockProjectPath)
      vi.mocked(fs.existsSync).mockReturnValue(true)

      integrator.reportBuildResult({
        success: true,
        duration_ms: 100,
        tool: "unity-build",
      })

      const results = integrator.getRecentResults()
      expect(results).toHaveLength(1)
      expect(results[0]).toMatchObject({
        type: "build",
        success: true,
        tool: "unity-build",
      })
      expect(fs.appendFileSync).toHaveBeenCalledWith(
        expect.stringContaining("execution-log.jsonl"),
        expect.stringContaining('"type":"build"'),
        "utf-8"
      )
    })

    it("should report test result", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportTestResult({
        passed: 10,
        failed: 2,
        skipped: 0,
      })

      const results = integrator.getRecentResults()
      expect(results[0]).toMatchObject({
        type: "test",
        success: false,
        passed: 10,
        failed: 2,
      })
    })

    it("should report tool execution", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({
        tool: "screenshot",
        success: true,
      })

      const results = integrator.getRecentResults()
      expect(results[0]).toMatchObject({
        type: "tool",
        tool: "screenshot",
        success: true,
      })
    })
  })

  describe("health and pausing", () => {
    it("should calculate health score based on window", () => {
      const integrator = createExternalSignalIntegrator()
      for (let i = 0; i < 10; i++) {
        integrator.reportToolExecution({ tool: "t", success: i < 7 })
      }
      expect(integrator.getHealthScore()).toBe(70)
    })

    it("should suggest pausing if failure rate > 50% in window", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "t1", success: false })
      integrator.reportToolExecution({ tool: "t2", success: false })
      integrator.reportToolExecution({ tool: "t3", success: true })

      expect(integrator.shouldPauseForErrors()).toBe(true)
    })

    it("should not suggest pausing if history is too short", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "t1", success: false })
      integrator.reportToolExecution({ tool: "t2", success: false })
      expect(integrator.shouldPauseForErrors()).toBe(false)
    })
  })

  describe("action suggestions", () => {
    it("should suggest continuing if no failures", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "t", success: true })
      expect(integrator.getNextActionSuggestion()).toContain("Continue")
    })

    it("should suggest fixing build errors", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportBuildResult({
        success: false,
        duration_ms: 0,
        errors: ["CS0103: The name 'x' does not exist"],
      })
      const suggestion = integrator.getNextActionSuggestion()
      expect(suggestion).toContain("Fix build errors")
      expect(suggestion).toContain("CS0103")
    })

    it("should handle build failures without error details", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportBuildResult({ success: false, duration_ms: 0, errors: [] })

      expect(integrator.getNextActionSuggestion()).toBe("Fix build errors before continuing.")
    })

    it("should suggest fixing test failures", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportTestResult({ passed: 5, failed: 3, skipped: 0 })
      const suggestion = integrator.getNextActionSuggestion()
      expect(suggestion).toContain("3 tests failing")
      expect(suggestion).toContain("Fix failing tests")
    })

    it("should suggest investigating generic tool failure", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "custom-tool", success: false })
      expect(integrator.getNextActionSuggestion()).toContain('"custom-tool" failed')
    })

    it("should truncate build errors to three lines", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportBuildResult({ success: false, duration_ms: 0, errors: ["E1", "E2", "E3", "E4"] })

      const suggestion = integrator.getNextActionSuggestion()
      expect(suggestion).toContain("E1")
      expect(suggestion).toContain("E3")
      expect(suggestion).not.toContain("E4")
    })

    it("should return the last generic tool failure", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "tool-a", success: false })
      integrator.reportToolExecution({ tool: "tool-b", success: false })
      expect(integrator.getNextActionSuggestion()).toContain('"tool-b" failed')
    })
  })

  describe("history management", () => {
    it("reports malformed JSONL records as observable errors", () => {
      const consoleError = vi.spyOn(console, "error").mockImplementation(() => undefined)
      vi.mocked(fs.readFileSync).mockReturnValue('{"type":"tool","tool":"ok","success":true,"timestamp":1}\nnot-json\n')

      const result = readExecutionLogRecords(mockLogPath)

      expect(result.records).toHaveLength(1)
      expect(result.errors).toHaveLength(1)
      expect(consoleError).toHaveBeenCalledWith(
        expect.stringContaining("malformed execution log record"),
        expect.any(Error),
      )
      consoleError.mockRestore()
    })

    it("should clear history", () => {
      const integrator = createExternalSignalIntegrator()
      integrator.reportToolExecution({ tool: "t", success: true })
      integrator.clearHistory()
      expect(integrator.getRecentResults()).toHaveLength(0)
    })

    it("should limit history size", () => {
      const integrator = createExternalSignalIntegrator()
      for (let i = 0; i < 35; i++) {
        integrator.reportToolExecution({ tool: `t${i}`, success: true })
      }
      expect(integrator.getRecentResults().length).toBeLessThanOrEqual(30)
    })

    it("should handle persistence failure gracefully", () => {
      const integrator = createExternalSignalIntegrator(mockProjectPath)
      vi.mocked(fs.appendFileSync).mockImplementation(() => {
        throw new Error("Disk full")
      })

      expect(() => {
        integrator.reportToolExecution({ tool: "t", success: true })
      }).not.toThrow()
    })

    it("should clear history on destroy", () => {
      const integrator = createExternalSignalIntegrator(mockProjectPath)
      integrator.reportToolExecution({ tool: "t", success: true })
      integrator.destroy()
      expect(integrator.getRecentResults()).toEqual([])
    })
  })
})
