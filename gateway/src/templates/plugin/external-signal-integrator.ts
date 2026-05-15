import * as fs from "node:fs"
import { join } from "node:path"

export interface BuildResult {
  success: boolean
  errors?: string[]
  duration_ms: number
  tool?: string
  timestamp?: number
}

export interface TestResult {
  passed: number
  failed: number
  skipped: number
  output?: string
  tool?: string
  timestamp?: number
}

export interface ToolExecutionResult {
  tool: string
  success: boolean
  output?: string
  timestamp?: number
}

export interface ExecutionRecord extends ToolExecutionResult {
  type: "build" | "test" | "tool"
  timestamp: number
  errors?: string[]
  duration_ms?: number
  passed?: number
  failed?: number
  skipped?: number
}

export interface ExternalSignalIntegrator {
  reportBuildResult(result: BuildResult): void
  reportTestResult(result: TestResult): void
  reportToolExecution(result: ToolExecutionResult): void
  getHealthScore(): number
  shouldPauseForErrors(): boolean
  getNextActionSuggestion(): string
  getRecentResults(): ExecutionRecord[]
  clearHistory(): void
  destroy(): void
}

export interface ExecutionLogReadResult {
  records: ExecutionRecord[]
  errors: Error[]
}

const DEFAULT_WINDOW_SIZE = 10
const DEFAULT_LOG_PATH = ".lux/execution-log.jsonl"

function isExecutionRecord(value: unknown): value is ExecutionRecord {
  if (typeof value !== "object" || value === null) return false
  const record = value as Record<string, unknown>
  return (
    (record.type === "build" || record.type === "test" || record.type === "tool") &&
    typeof record.tool === "string" &&
    typeof record.success === "boolean" &&
    typeof record.timestamp === "number"
  )
}

export function readExecutionLogRecords(logPath: string): ExecutionLogReadResult {
  const content = fs.readFileSync(logPath, "utf-8")
  const records: ExecutionRecord[] = []
  const errors: Error[] = []

  content.split(/\r?\n/).forEach((line, index) => {
    if (line.trim().length === 0) return

    try {
      const parsed = JSON.parse(line) as unknown
      if (!isExecutionRecord(parsed)) {
        throw new Error(`invalid execution record shape at line ${index + 1}`)
      }
      records.push(parsed)
    } catch (err) {
      const error = err instanceof Error ? err : new Error(String(err))
      errors.push(error)
      console.error(`[lux-external-signal] malformed execution log record at ${logPath}:${index + 1}`, error)
    }
  })

  return { records, errors }
}

export function createExternalSignalIntegrator(projectPath?: string): ExternalSignalIntegrator {
  let history: ExecutionRecord[] = []
  let logPath = projectPath ? join(projectPath, DEFAULT_LOG_PATH) : ""

  function persistRecord(record: ExecutionRecord): void {
    if (!logPath) return

    try {
      const dirPath = join(projectPath as string, ".lux")
      if (!fs.existsSync(dirPath)) {
        fs.mkdirSync(dirPath, { recursive: true })
      }

      const line = JSON.stringify(record)
      fs.appendFileSync(logPath, `${line}\n`, "utf-8")
    } catch { /* intentional: persistence failure must not block continuation decisions */
    }
  }

  function appendRecord(record: ExecutionRecord): void {
    history.push(record)

    if (history.length > DEFAULT_WINDOW_SIZE * 3) {
      history = history.slice(-DEFAULT_WINDOW_SIZE * 2)
    }

    persistRecord(record)
  }

  function getWindow(size = DEFAULT_WINDOW_SIZE): ExecutionRecord[] {
    return history.slice(-size)
  }

  function getHealthScore(): number {
    const window = getWindow()
    if (window.length === 0) return 100

    const passed = window.filter((record) => record.success).length
    return Math.round((passed / window.length) * 100)
  }

  function shouldPauseForErrors(): boolean {
    const window = getWindow()
    if (window.length < 3) return false

    const recentFailures = window.filter((record) => !record.success).length
    return recentFailures / window.length > 0.5
  }

  function getNextActionSuggestion(): string {
    const window = getWindow()
    const failures = window.filter((record) => !record.success)

    if (failures.length === 0) {
      return "All recent executions passing. Continue with next ticket."
    }

    const buildFailures = failures.filter((record) => record.type === "build")
    if (buildFailures.length > 0) {
      const lastBuild = buildFailures[buildFailures.length - 1]
      const errors = lastBuild.errors ?? []
      const errorList = errors.slice(0, 3).map((error) => `  - ${error}`).join("\n")
      return errorList.length > 0
        ? `Fix build errors before continuing:\n${errorList}`
        : "Fix build errors before continuing."
    }

    const testFailures = failures.filter((record) => record.type === "test")
    if (testFailures.length > 0) {
      const lastTest = testFailures[testFailures.length - 1]
      return `${lastTest.failed ?? 0} tests failing (${lastTest.passed ?? 0} passed). Fix failing tests before continuing.`
    }

    const lastFailure = failures[failures.length - 1]
    return `"${lastFailure.tool}" failed. Investigate and fix before continuing.`
  }

  return {
    reportBuildResult(result: BuildResult): void {
      appendRecord({
        ...result,
        type: "build",
        tool: result.tool ?? "build",
        success: result.success,
        timestamp: result.timestamp ?? Date.now(),
      })
    },
    reportTestResult(result: TestResult): void {
      appendRecord({
        ...result,
        type: "test",
        tool: result.tool ?? "test",
        success: result.failed === 0,
        timestamp: result.timestamp ?? Date.now(),
      })
    },
    reportToolExecution(result: ToolExecutionResult): void {
      appendRecord({ ...result, type: "tool", timestamp: result.timestamp ?? Date.now() })
    },
    getHealthScore,
    shouldPauseForErrors,
    getNextActionSuggestion,
    getRecentResults: () => [...history],
    clearHistory: () => {
      history = []
    },
    destroy: () => {
      history = []
      logPath = ""
    },
  }
}
