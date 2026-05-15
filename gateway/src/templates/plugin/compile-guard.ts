const childProcess = require("node:child_process") as {
  execFileSync: (
    command: string,
    args: string[],
    options: { timeout: number; encoding: "utf-8"; stdio: ["pipe", "pipe", "pipe"] },
  ) => string
}

export interface CompileError {
  file: string
  line?: number
  column?: number
  code?: string
  message: string
  severity: "error" | "warning"
}

export interface CompileOutput {
  stdout: string
  stderr: string
  exitCode: number
  success: boolean
  durationMs: number
}

export interface CompileCheckResult {
  hasErrors: boolean
  errors: CompileError[]
  warnings: CompileError[]
  wasFixed: boolean
  shouldRetry: boolean
  output: CompileOutput
}

interface OpenCodePluginServerContext {
  directory?: string
  client?: {
    session?: {
      promptAsync?: (input: {
        path: { id: string }
        body: { parts: Array<{ type: "text"; text: string }> }
        query: { directory: string }
      }) => Promise<unknown>
    }
  }
}

interface CompileGuardConfig {
  projectPath: string
  gatewayUrl: string
  sessionID?: string
}

interface CompileGuardState {
  consecutiveCompileFailures: number
}

const MAX_COMPILE_FIX_RETRIES = 3
const COMPILE_TIMEOUT_MS = 120000

function textFromProcessBuffer(value: unknown): string {
  if (typeof value === "string") return value
  if (value && typeof value === "object" && "toString" in value) return String(value)
  return ""
}

function parseNumber(value: string | undefined): number | undefined {
  if (!value) return undefined
  const parsed = Number(value)
  return Number.isFinite(parsed) ? parsed : undefined
}

function makeResult(output: CompileOutput, diagnostics: CompileError[], wasFixed: boolean, shouldRetry: boolean): CompileCheckResult {
  const errors = diagnostics.filter((item) => item.severity === "error")
  const warnings = diagnostics.filter((item) => item.severity === "warning")
  return { hasErrors: errors.length > 0, errors, warnings, wasFixed, shouldRetry, output }
}

export async function checkAndFixCompile(
  ctx: OpenCodePluginServerContext,
  config: CompileGuardConfig,
  state: CompileGuardState,
): Promise<CompileCheckResult> {
  const output = runCompile(config.projectPath)
  const diagnostics = parseCompileErrors(output)
  const errors = diagnostics.filter((item) => item.severity === "error")

  if (errors.length === 0) return makeResult(output, diagnostics, false, false)
  if (state.consecutiveCompileFailures >= MAX_COMPILE_FIX_RETRIES) {
    console.warn("[Lux] Compile auto-fix retries exhausted", {
      projectPath: config.projectPath,
      errorCount: errors.length,
      retries: state.consecutiveCompileFailures,
    })
    return makeResult(output, diagnostics, false, false)
  }

  const promptAsync = ctx.client?.session?.promptAsync
  if (!promptAsync) {
    console.warn("[Lux] Compile auto-fix unavailable: promptAsync missing", { projectPath: config.projectPath })
    return makeResult(output, diagnostics, false, true)
  }

  await promptAsync({
    path: { id: config.sessionID ?? "lux-session" },
    body: { parts: [{ type: "text", text: buildFixPrompt(errors) }] },
    query: { directory: ctx.directory ?? config.projectPath },
  })
  state.consecutiveCompileFailures += 1
  return makeResult(output, diagnostics, true, true)
}

export function runCompile(projectPath: string): CompileOutput {
  const start = Date.now()
  try {
    const stdout = childProcess.execFileSync("lux", ["compile", "--project-path", projectPath], {
      timeout: COMPILE_TIMEOUT_MS,
      encoding: "utf-8",
      stdio: ["pipe", "pipe", "pipe"],
    })
    return { stdout, stderr: "", exitCode: 0, success: true, durationMs: Date.now() - start }
  } catch (err: unknown) {
    const error = err as { stdout?: unknown; stderr?: unknown; status?: number; signal?: string }
    return {
      stdout: textFromProcessBuffer(error.stdout),
      stderr: textFromProcessBuffer(error.stderr),
      exitCode: error.status ?? 1,
      success: false,
      durationMs: Date.now() - start,
    }
  }
}

export function parseCompileErrors(output: CompileOutput): CompileError[] {
  const diagnostics: CompileError[] = []
  const seen = new Set<string>()
  const text = `${output.stdout}\n${output.stderr}`
  const pattern = /^(?:(?<file>.+?)(?:\((?<line>\d+)(?:,(?<column>\d+))?\)|\[(?<bracketLine>\d+)(?:,(?<bracketColumn>\d+))?\])\s*:\s*)?(?<severity>error|warning)\s+(?<code>(?:CS|UCE)\d+)\s*:\s*(?<message>.+)$/gim

  for (const match of text.matchAll(pattern)) {
    const groups = match.groups ?? {}
    const severity = groups.severity?.toLowerCase() === "warning" ? "warning" : "error"
    const diagnostic: CompileError = {
      file: (groups.file ?? "<unknown>").trim(),
      line: parseNumber(groups.line ?? groups.bracketLine),
      column: parseNumber(groups.column ?? groups.bracketColumn),
      code: groups.code,
      message: (groups.message ?? "").trim(),
      severity,
    }
    const key = `${diagnostic.severity}|${diagnostic.file}|${diagnostic.line ?? ""}|${diagnostic.column ?? ""}|${diagnostic.code ?? ""}|${diagnostic.message}`
    if (!seen.has(key)) {
      seen.add(key)
      diagnostics.push(diagnostic)
    }
  }

  return diagnostics
}

export function buildFixPrompt(errors: CompileError[]): string {
  const lines = errors.map((error) => {
    const code = error.code ? `**${error.code}** ` : ""
    const location = `${error.file}${error.line ? `:${error.line}` : ""}${error.column ? `:${error.column}` : ""}`
    return `- ${code}${location}: ${error.message}`
  })

  return [
    `## Compile Errors Detected (${errors.length} errors)`,
    "",
    "Fix ALL of the following Unity C# compilation errors in the project:",
    "",
    lines.join("\n"),
    "",
    "Rules:",
    "- Fix only the files mentioned above",
    "- Do NOT refactor unrelated code",
    "- Do NOT add new features",
    "- Ensure 0 compile errors remain after your fixes",
  ].join("\n")
}
