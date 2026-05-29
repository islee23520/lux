import { createProgressPoller } from "../../gateway/src/templates/plugin/progress-poller"

declare const require: (moduleName: string) => {
  readFileSync?: (filePath: string, encoding: "utf-8") => string
  readdirSync?: (dirPath: string) => string[]
  join?: (...parts: string[]) => string
}
declare const process: { cwd: () => string; env?: Record<string, string | undefined> }

const fs = require("node:fs") as {
  readFileSync: (filePath: string, encoding: "utf-8") => string
  readdirSync: (dirPath: string) => string[]
}
const path = require("node:path") as {
  join: (...parts: string[]) => string
}
const continuationStateClient = require("../../gateway/src/templates/plugin/continuation-state-client") as {
  readContinuationState: (projectPath: string) => {
    session_id: string | null
    continuation_count: number
    stagnation_count: number
    consecutive_failures: number
    last_ambiguity: string | null
    last_ticket_baseline: string | null
    current_ticket_id: string | null
    status: "Idle" | "Active" | "Stopped" | "Error" | "Complete"
    started_at: string | null
    updated_at: string
    stop_reason: string | null
  }
  writeContinuationState: (projectPath: string, state: ContinuationState) => void
  updateContinuationState: (projectPath: string, partial: Record<string, unknown>) => unknown
}
const ticketLoader = require("../../gateway/src/templates/plugin/ticket-loader") as {
  getTicketById: (projectPath: string, id: string) => Ticket | null
  invalidateCache: () => void
  loadTickets: (projectPath: string) => TicketSummary
}
const stagnationDetection = require("../../gateway/src/templates/plugin/stagnation-detection") as {
  trackProgress: (state: LuxSessionState, tickets: Ticket[], ambiguity: number, integrator?: ExternalSignalIntegrator) => void
  shouldStopForStagnation: (state: LuxSessionState, maxStagnationOrIntegrator?: number | ExternalSignalIntegrator, integrator?: ExternalSignalIntegrator) => boolean
  getStagnationDetails: (state: LuxSessionState, integratorOrMaxStagnation?: ExternalSignalIntegrator | number, maxStagnation?: number) => StagnationDetails
}
const nextActionGenerator = require("../../gateway/src/templates/plugin/next-action-generator") as {
  generateNextAction: (ctx: NextActionContext) => NextActionResult
}
const stopEvaluator = require("../../gateway/src/templates/plugin/stop-evaluator") as {
  evaluateStopConditions: (ctx: StopEvaluationContext) => StopDecision
}
const promptBuilder = require("../../gateway/src/templates/plugin/prompt-builder") as {
  buildContinuationPrompt: (ctx: PromptBuilderContext) => string
}
const errorRecovery = require("../../gateway/src/templates/plugin/error-recovery") as {
  classifyError: (error: unknown) => ErrorClassification
  handlePromptAsyncError: (error: unknown, state: RecoveryState) => RecoveryDecision
  getBackoffDelayMs: (errorType: ErrorType, consecutiveCount: number) => number
}
const externalSignals = require("../../gateway/src/templates/plugin/external-signal-integrator") as {
  createExternalSignalIntegrator: (projectPath?: string) => ExternalSignalIntegrator
}
const luxOverlay = require("../../gateway/src/templates/plugin/lux-overlay") as {
  buildToastMessage: typeof import("../../gateway/src/templates/plugin/lux-overlay").buildToastMessage
  formatContextBlock: typeof import("../../gateway/src/templates/plugin/lux-overlay").formatContextBlock
  formatStatus: typeof import("../../gateway/src/templates/plugin/lux-overlay").formatStatus
}
const gatewaySpawn = require("../../gateway/src/templates/plugin/gateway-spawn") as {
  ensureGatewayRunning: (config: GatewaySpawnConfig) => Promise<GatewaySpawnResult>
}
const sessionEnd = require("../../gateway/src/templates/plugin/session-end-detector") as {
  setupSessionEndDetection: (
    options: { registerEvent: (handler: (event: Record<string, unknown>) => void) => void | (() => void); projectPath?: string; getProcessing?: () => boolean },
    onSessionEnd: (ctx: SessionEndContext) => Promise<void> | void,
  ) => SessionEndDetectorControls
}
const compileGuard = require("../../gateway/src/templates/plugin/compile-guard") as {
  checkAndFixCompile: (ctx: LuxPluginContext, config: { projectPath: string; gatewayUrl: string; sessionID?: string }, state: { consecutiveCompileFailures: number }) => Promise<CompileCheckResult>
}
const { invalidateCache, loadTickets: loadTicketsFromDisk } = ticketLoader
const { trackProgress, shouldStopForStagnation, getStagnationDetails } = stagnationDetection
const { generateNextAction } = nextActionGenerator
const { evaluateStopConditions } = stopEvaluator
const { buildContinuationPrompt } = promptBuilder
const { classifyError, handlePromptAsyncError, getBackoffDelayMs } = errorRecovery
const { createExternalSignalIntegrator } = externalSignals
const { buildToastMessage: _buildToastMessage, formatContextBlock: _formatContextBlock, formatStatus: _formatStatus } = luxOverlay
const { ensureGatewayRunning } = gatewaySpawn
const { setupSessionEndDetection } = sessionEnd
const { checkAndFixCompile } = compileGuard

type LuxPluginContext = {
  project?: string
  client?: {
    session?: {
      promptAsync?: (input: {
        path: { id: string }
        body: { parts: Array<{ type: "text"; text: string }> }
        query: { directory: string }
      }) => Promise<unknown>
    }
    tui?: {
      showToast?: (input: { body: { message: string; variant: "success" | "error" | "info" } }) => Promise<unknown> | unknown
    }
    app?: {
      log?: (input: {
        body: {
          service: string
          level: "debug" | "info" | "warn" | "error"
          message: string
          extra?: Record<string, unknown>
        }
      }) => Promise<unknown> | unknown
    }
  }
  directory?: string
  worktree?: string
}

type OpenCodeEvent = {
  type?: string
  properties?: Record<string, unknown>
  [key: string]: unknown
}

type ToolInput = {
  tool?: string
  [key: string]: unknown
}

type CompactingOutput = {
  context?: string[]
  [key: string]: unknown
}

type Ticket = {
  id?: string
  title?: string
  description?: string
  status: string
  priority?: string
  spec_ref?: string | null
  tags?: string[]
  blockers?: unknown[]
  acceptance_criteria?: unknown
  [key: string]: unknown
}

type TicketSummary = {
  tickets: Ticket[]
  byStatus: Record<string, number>
  activeTickets: Ticket[]
  incompleteCount: number
}

type LuxEvalResult = {
  should_continue: boolean
  next_action: string
  ambiguity_score: number
  continuation_count: number
}

type LuxSessionState = {
  continuationCount: number
  lastInjectedAt: number
  awaitingPostInjectionProgressCheck: boolean
  inFlight: boolean
  stagnationCount: number
  consecutiveFailures: number
  consecutiveCompileFailures: number
  recentCompactionAt: number | null
  recentCompactionEpoch: number
  acknowledgedCompactionEpoch: number
  lastIncompleteTicketCount: number
  lastAmbiguityScore: number
  abortDetectedAt?: number
  tokenLimitDetected?: boolean
}

type ContinuationState = {
  session_id: string | null
  continuation_count: number
  stagnation_count: number
  consecutive_failures: number
  last_ambiguity: string | null
  last_ticket_baseline: string | null
  current_ticket_id: string | null
  status: "Idle" | "Active" | "Stopped" | "Error" | "Complete"
  started_at: string | null
  updated_at: string
  stop_reason: string | null
}

type StagnationDetails = {
  shouldStop: boolean
  reasons: string[]
  ticketStagnation: boolean
  healthScore?: number
  buildFailureStreak?: number
  zeroProgressCycles: number
}

type ExternalSignalIntegrator = {
  reportToolExecution: (result: { tool: string; success: boolean; output?: string; timestamp?: number }) => void
  getHealthScore: () => number
  getNextActionSuggestion: () => string
  getRecentResults: () => Array<{ tool: string; success: boolean; type: "build" | "test" | "tool"; timestamp: number; errors?: string[] }>
  destroy: () => void
}

type NextActionContext = {
  activeTickets: Array<{ id?: string; title?: string; status: string; priority?: string; spec_ref?: string | null }>
  inactiveTickets: Array<{ id?: string; title?: string; status: string }>
  ticketCounts: Record<string, number>
  incompleteCount: number
  ambiguityScore: number
  shouldContinueSpec: boolean
  nextSpecAction: string
  continuationCount: number
  stagnationCount: number
  consecutiveFailures: number
  lastAmbiguity: number
  healthScore?: number
  lastError?: string
  suggestedAction?: string
  isCompactionGuardActive: boolean
  maxContinuations: number
}

type NextActionResult = {
  message: string
  shouldInject: boolean
  confidence: number
  reason: string
}

type StopDecision = {
  shouldStop: boolean
  reason: StopReason
  confidence: number
}

type StopEvaluationContext = {
  state: LuxSessionState
  stagnationDetails: StagnationDetails
  activeTickets: Array<{ status: string; title?: string | null }>
  ambiguity: number
  continuationState: { status: string; consecutive_failures: number }
  clarificationTicketInProgress: boolean
  config?: Partial<{
    maxContinuations: number
    healthThreshold: number
    consecutiveFailuresThreshold: number
    ambiguityThreshold: number
  }>
}

type PromptBuilderContext = {
  ticket: Ticket
  nextAction: NextActionResult
  ambiguity: number
  summary: TicketSummary
  continuationCount: number
  consecutiveFailures: number
  lastError?: string
}

type ErrorType = "rate_limit" | "auth" | "network" | "invalid_session" | "timeout" | "unknown"

type ErrorClassification = {
  errorType: ErrorType
  message: string
  status: number | null
}

type RecoveryState = {
  consecutiveErrorsByType: Record<ErrorType, number>
  lastErrorTime: string | null
  lastErrorType: ErrorType | null
}

type RecoveryDecision = {
  action: "Retry" | "Abort" | "Backoff" | "Skip"
  delayMs: number
  errorType: ErrorType
}

type SessionEndContext = {
  projectPath: string
  incompleteCount: number
  reason: "session.end" | "session.shutdown" | "session.error" | "manual"
}

type SessionEndDetectorControls = {
  destroy: () => void
  triggerManualResume: () => void
  consumePendingResume: () => void
  setProcessing: (value: boolean) => void
  setProjectPath: (projectPath: string) => void
}

type GatewaySpawnConfig = {
  gatewayUrl: string
  projectPath: string
  healthTimeoutMs?: number
  healthIntervalMs?: number
  spawnCommand?: string
  spawnArgs?: string[]
}

type GatewaySpawnResult = {
  spawned: boolean
  pid?: number
  readyMs: number
}

type CompileCheckResult = {
  hasErrors: boolean
  errors: Array<{ severity: "error" | "warning" }>
  warnings: Array<{ severity: "error" | "warning" }>
  wasFixed: boolean
  shouldRetry: boolean
}

export interface OrchestratorConfig {
  projectPath: string
  gatewayUrl: string
  maxContinuations?: number
  minContinuationIntervalMs?: number
  healthThreshold?: number
  maxStagnation?: number
}

export interface OrchestratorDeps {
  stateClient: typeof continuationStateClient
  ticketLoader: typeof ticketLoader
  signalIntegrator: () => ExternalSignalIntegrator | undefined
}

export type StopReason =
  | "max_continations"
  | "user_abort"
  | "stagnation"
  | "health_critical"
  | "all_complete"
  | "ambiguity_too_high"
  | "consecutive_state_error"
  | null

export interface CycleResult {
  dispatched: boolean
  stopReason: StopReason
  selectedTicketId: string | null
  message: string
}

const SERVICE = "lux-plugin"
const MAX_CONTINUATIONS = 50
const TARGET_AMBIGUITY = 0.02
const CONTINUATION_COOLDOWN_MS = 3_000
const MAX_STAGNATION = 3
const MAX_CONSECUTIVE_FAILURES = 3
const COMPACTION_GUARD_MS = 60_000

const states = new Map<string, LuxSessionState>()
const MUTATING_LUX_TOOLS = new Set([
  "lux_init",
  "lux_spec_apply",
  "lux_spec_update",
  "lux_ticket_create",
  "lux_ticket_update",
  "lux_ticket_status",
  "lux_verify",
])

const eventProperties = (event: OpenCodeEvent) => event.properties ?? event

const valueToString = (value: unknown) => {
  if (typeof value === "string") return value
  if (value === undefined || value === null) return ""
  return String(value)
}

const resultStatus = (output: unknown) => {
  if (!output || typeof output !== "object") return "completed"

  const record = output as Record<string, unknown>
  if (typeof record.status === "string") return record.status
  if (typeof record.error === "string" || record.error) return "error"
  if (typeof record.ok === "boolean") return record.ok ? "success" : "error"

  return "completed"
}

function stateFor(projectPath: string): LuxSessionState {
  let state = states.get(projectPath)
  if (!state) {
    state = {
      continuationCount: 0,
      lastInjectedAt: 0,
      awaitingPostInjectionProgressCheck: false,
      inFlight: false,
      stagnationCount: 0,
      consecutiveFailures: 0,
      consecutiveCompileFailures: 0,
      recentCompactionAt: null,
      recentCompactionEpoch: 0,
      acknowledgedCompactionEpoch: 0,
      lastIncompleteTicketCount: -1,
      lastAmbiguityScore: Number.POSITIVE_INFINITY,
    }
    states.set(projectPath, state)
  }
  return state
}

function resolveSessionID(event: OpenCodeEvent): string {
  const properties = eventProperties(event)
  for (const key of ["sessionID", "sessionId", "id"]) {
    const value = properties[key]
    if (typeof value === "string" && value.length > 0) return value
  }
  return "lux-session"
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function loadTickets(projectPath: string): TicketSummary {
  return loadTicketsFromDisk(projectPath)
}

function loadSpec(projectPath: string): Record<string, unknown> | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(projectPath, ".lux", "spec.json"), "utf-8")) as Record<string, unknown>
  } catch {
    /* intentional: missing spec falls back to ambiguity-driven continuation */
    return null
  }
}

function evaluateSpec(projectPath: string): LuxEvalResult {
  const spec = loadSpec(projectPath)
  if (!spec) {
    return {
      should_continue: true,
      next_action: "No spec.json found. Run Lux initialization or define .lux/spec.json first.",
      ambiguity_score: 1,
      continuation_count: 0,
    }
  }

  const issues: string[] = []
  let totalChecks = 0
  let passedChecks = 0

  const domains = isRecord(spec.domains) ? spec.domains : {}
  for (const domain of ["design", "architecture", "art_style", "audio", "narrative", "levels", "ui_ux"]) {
    totalChecks += 1
    const domainSpec = domains[domain]
    if (isRecord(domainSpec) && domainSpec.defined === true) passedChecks += 1
    else issues.push(`Define the ${domain} spec domain.`)
  }

  totalChecks += 1
  if (isRecord(spec.testing) && typeof spec.testing.framework === "string" && spec.testing.framework.length > 0) passedChecks += 1
  else issues.push("Specify the Lux test framework/strategy.")

  const ambiguity = typeof spec.overall_ambiguity === "number"
    ? spec.overall_ambiguity
    : totalChecks > 0
      ? 1 - passedChecks / totalChecks
      : 1

  return {
    should_continue: ambiguity > TARGET_AMBIGUITY || issues.length > 0,
    next_action: issues[0] ?? "Continue the next active Lux ticket.",
    ambiguity_score: Math.round(ambiguity * 100) / 100,
    continuation_count: 0,
  }
}

function isCompactionGuardActive(state: LuxSessionState): boolean {
  if (state.recentCompactionAt === null) return false
  if (state.acknowledgedCompactionEpoch >= state.recentCompactionEpoch) return false
  return Date.now() - state.recentCompactionAt < COMPACTION_GUARD_MS
}

function armCompactionGuard(state: LuxSessionState): void {
  state.recentCompactionAt = Date.now()
  state.recentCompactionEpoch += 1
}

function priorityRank(priority: string | undefined): number {
  const ranks: Record<string, number> = { Critical: 0, High: 1, Medium: 2, Low: 3 }
  return ranks[priority ?? ""] ?? 99
}

function selectBestTicket(tickets: Ticket[]): Ticket | null {
  const active = tickets.filter((ticket) => ticket.status !== "Done" && ticket.status !== "Blocked")
  if (active.length === 0) return null

  const inProgress = active.filter((ticket) => ticket.status === "InProgress")
  if (inProgress.length > 0) {
    return inProgress.sort((a, b) => priorityRank(a.priority) - priorityRank(b.priority))[0] ?? null
  }

  const todo = active.filter((ticket) => ticket.status === "ToDo" || ticket.status === "Todo")
  return todo
    .sort((a, b) => {
      const pDiff = priorityRank(a.priority) - priorityRank(b.priority)
      if (pDiff !== 0) return pDiff
      return (a.blockers?.length ?? 0) - (b.blockers?.length ?? 0)
    })[0] ?? null
}

function createRecoveryState(): RecoveryState {
  return {
    consecutiveErrorsByType: {
      rate_limit: 0,
      auth: 0,
      network: 0,
      invalid_session: 0,
      timeout: 0,
      unknown: 0,
    },
    lastErrorTime: null,
    lastErrorType: null,
  }
}

export class ContinuationOrchestrator {
  private readonly config: Required<OrchestratorConfig>
  private readonly deps: OrchestratorDeps
  private readonly ctx: LuxPluginContext
  private readonly getSessionID: () => string
  private readonly logDecision: (level: "debug" | "info" | "warn" | "error", message: string, extra?: Record<string, unknown>) => Promise<void>
  private readonly recoveryState: RecoveryState = createRecoveryState()
  private state: LuxSessionState
  private lastDispatchTime = 0

  constructor(args: {
    config: OrchestratorConfig
    deps: OrchestratorDeps
    ctx: LuxPluginContext
    state: LuxSessionState
    getSessionID: () => string
    logDecision: (level: "debug" | "info" | "warn" | "error", message: string, extra?: Record<string, unknown>) => Promise<void>
  }) {
    this.config = {
      projectPath: args.config.projectPath,
      gatewayUrl: args.config.gatewayUrl,
      maxContinuations: args.config.maxContinuations ?? MAX_CONTINUATIONS,
      minContinuationIntervalMs: args.config.minContinuationIntervalMs ?? CONTINUATION_COOLDOWN_MS,
      healthThreshold: args.config.healthThreshold ?? 20,
      maxStagnation: args.config.maxStagnation ?? MAX_STAGNATION,
    }
    this.deps = args.deps
    this.ctx = args.ctx
    this.state = args.state
    this.getSessionID = args.getSessionID
    this.logDecision = args.logDecision
  }

  isProcessing(): boolean {
    return this.state.inFlight
  }

  private result(dispatched: boolean, stopReason: StopReason, selectedTicketId: string | null, message: string): CycleResult {
    return { dispatched, stopReason, selectedTicketId, message }
  }

  private persist(contState: ContinuationState, patch: Partial<ContinuationState>): ContinuationState {
    const next = { ...contState, ...patch }
    this.deps.stateClient.writeContinuationState(this.config.projectPath, next)
    return next
  }

  async onTrigger(reason: string): Promise<CycleResult> {
    const now = Date.now()
    const backoff = this.recoveryState.lastErrorType
      ? getBackoffDelayMs(this.recoveryState.lastErrorType, this.recoveryState.consecutiveErrorsByType[this.recoveryState.lastErrorType])
      : 0
    const interval = Math.max(this.config.minContinuationIntervalMs, backoff)
    if (now - this.lastDispatchTime < interval) return this.result(false, null, null, "rate_limited")

    let contState = this.deps.stateClient.readContinuationState(this.config.projectPath)
    if (contState.status === "Stopped") return this.result(false, contState.stop_reason as StopReason, contState.current_ticket_id, "stopped")

    const summary = this.deps.ticketLoader.loadTickets(this.config.projectPath)
    const evalResult = evaluateSpec(this.config.projectPath)
    const ambiguity = evalResult.ambiguity_score
    const integrator = this.deps.signalIntegrator()
    trackProgress(this.state, summary.tickets, ambiguity, integrator)
    const healthScore = integrator?.getHealthScore()
    const stagnation = getStagnationDetails(this.state, integrator, this.config.maxStagnation)
    const recoveryAction = integrator?.getNextActionSuggestion()
    const hasRecoveryAction = Boolean(recoveryAction && !recoveryAction.toLowerCase().includes("continue with next ticket"))
    const activeTickets = summary.tickets.filter((ticket) => ticket.status !== "Done" && ticket.status !== "Blocked")
    const hasClarificationInProgress = summary.tickets.some((ticket) => ticket.status === "InProgress" && (ticket.title ?? "").toLowerCase().includes("clarify"))
    const stopDecision = evaluateStopConditions({
      state: this.state,
      stagnationDetails: {
        ...stagnation,
        shouldStop: (stagnation.shouldStop || shouldStopForStagnation(this.state, this.config.maxStagnation, integrator)) && !hasRecoveryAction,
        healthScore,
      },
      activeTickets,
      ambiguity,
      continuationState: { status: contState.status, consecutive_failures: contState.consecutive_failures },
      clarificationTicketInProgress: hasClarificationInProgress,
      config: {
        maxContinuations: this.config.maxContinuations,
        healthThreshold: this.config.healthThreshold,
        consecutiveFailuresThreshold: MAX_CONSECUTIVE_FAILURES,
        ambiguityThreshold: 0.9,
      },
    })
    if (stopDecision.shouldStop && stopDecision.reason) {
      const _stopLevel = stopDecision.reason === "all_complete" ? ("info" as const) : ("warn" as const)
      const _stopToast = _buildToastMessage(_stopLevel, [
        `⛔ Lux stopped: ${stopDecision.reason}`,
        _formatStatus(
          { status: contState.status, continuationCount: this.state.continuationCount, consecutiveFailures: this.state.consecutiveFailures, currentTicketId: contState.current_ticket_id, stopReason: stopDecision.reason },
          { byStatus: summary.byStatus, activeTicketsCount: summary.activeTickets.length, incompleteCount: summary.incompleteCount },
        ),
      ])
      void this.ctx.client?.tui?.showToast?.({ body: _stopToast })
      if (stopDecision.reason === "all_complete") {
        contState = this.persist(contState, { status: "Complete", stop_reason: "all_complete" })
        return this.result(false, "all_complete", null, "all_complete")
      }
      if (stopDecision.reason === "consecutive_state_error") {
        return this.result(false, stopDecision.reason, contState.current_ticket_id, "continuation_state_error")
      }
      if (stopDecision.reason === "stagnation") {
        this.lastDispatchTime = now - this.config.minContinuationIntervalMs + getBackoffDelayMs("unknown", Math.max(1, stagnation.reasons.length))
      }
      contState = this.persist(contState, { status: "Stopped", stop_reason: stopDecision.reason, stagnation_count: this.state.stagnationCount, consecutive_failures: this.state.consecutiveFailures })
      return this.result(false, stopDecision.reason, contState.current_ticket_id, stopDecision.reason === "max_continations" ? "max_continuations" : stopDecision.reason)
    }
    if (isCompactionGuardActive(this.state)) return this.result(false, null, contState.current_ticket_id, "compaction_guard")
    if (this.state.inFlight || this.state.consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) return this.result(false, null, contState.current_ticket_id, "blocked")

    const ticket = selectBestTicket(summary.tickets)
    if (!ticket) return this.result(false, null, null, "no_ticket")

    const nextAction = generateNextAction({
      activeTickets: [ticket, ...activeTickets.filter((item) => item !== ticket)],
      inactiveTickets: summary.tickets.filter((item) => item.status === "Done" || item.status === "Blocked"),
      ticketCounts: summary.byStatus,
      incompleteCount: summary.incompleteCount,
      ambiguityScore: ambiguity,
      shouldContinueSpec: evalResult.should_continue,
      nextSpecAction: evalResult.next_action,
      continuationCount: this.state.continuationCount,
      stagnationCount: this.state.stagnationCount,
      consecutiveFailures: this.state.consecutiveFailures,
      lastAmbiguity: this.state.lastAmbiguityScore,
      healthScore,
      suggestedAction: recoveryAction,
      isCompactionGuardActive: isCompactionGuardActive(this.state),
      maxContinuations: this.config.maxContinuations,
    })
    if (!nextAction.shouldInject) return this.result(false, null, ticket.id ?? null, nextAction.message)

    const promptAsync = this.ctx.client?.session?.promptAsync
    if (!promptAsync) {
      this.state.consecutiveFailures += 1
      this.state.lastInjectedAt = Date.now()
      this.persist(contState, { status: "Error", consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount })
      return this.result(false, null, ticket.id ?? null, "promptAsync unavailable")
    }

    const message = buildContinuationPrompt({
      ticket,
      nextAction,
      ambiguity,
      summary,
      continuationCount: this.state.continuationCount,
      consecutiveFailures: this.state.consecutiveFailures,
    })
    this.state.inFlight = true
    try {
      await promptAsync({
        path: { id: this.getSessionID() },
        body: { parts: [{ type: "text", text: message }] },
        query: { directory: this.ctx.directory ?? this.ctx.worktree ?? process.cwd() },
      })
      this.state.inFlight = false
      this.lastDispatchTime = Date.now()
      this.state.lastInjectedAt = this.lastDispatchTime
      this.state.awaitingPostInjectionProgressCheck = true
      this.state.consecutiveFailures = 0
      this.state.continuationCount += 1
      contState = this.persist(contState, {
        session_id: this.getSessionID(),
        continuation_count: this.state.continuationCount,
        stagnation_count: this.state.stagnationCount,
        consecutive_failures: this.state.consecutiveFailures,
        last_ambiguity: String(ambiguity),
        last_ticket_baseline: String(summary.incompleteCount),
        current_ticket_id: ticket.id ?? contState.current_ticket_id,
        status: "Active",
        stop_reason: null,
        started_at: contState.started_at ?? new Date().toISOString(),
      })
      this.deps.ticketLoader.invalidateCache()
      await this.logDecision("info", "Lux continuation decision", {
        reason,
        dispatched: true,
        selectedTicketId: ticket.id ?? null,
        nextActionReason: nextAction.reason,
        healthScore,
        ambiguityScore: ambiguity,
        activeTicketCount: activeTickets.length,
        incompleteTicketCount: summary.incompleteCount,
        continuationCount: this.state.continuationCount,
        stagnationCount: this.state.stagnationCount,
      })
      const _dispatchToast = _buildToastMessage("info", [
        _formatStatus(
          { status: contState.status, continuationCount: this.state.continuationCount, consecutiveFailures: this.state.consecutiveFailures, currentTicketId: ticket.id ?? null },
          { byStatus: summary.byStatus, activeTicketsCount: summary.activeTickets.length, incompleteCount: summary.incompleteCount, totalTickets: summary.tickets.length },
          { dispatched: true, reason, selectedTicketId: ticket.id ?? null, healthScore, ambiguityScore: ambiguity, activeTicketCount: activeTickets.length, incompleteTicketCount: summary.incompleteCount, stagnationCount: this.state.stagnationCount, continuationCount: this.state.continuationCount },
        ),
      ])
      void this.ctx.client?.tui?.showToast?.({ body: _dispatchToast })
      return this.result(true, null, ticket.id ?? null, message)
    } catch (err) {
      this.state.inFlight = false
      this.state.lastInjectedAt = Date.now()
      this.state.consecutiveFailures += 1
      const classification = classifyError(err)
      const recoveryDecision = handlePromptAsyncError(err, this.recoveryState)
      if (recoveryDecision.action === "Abort") {
        this.persist(contState, { status: "Stopped", stop_reason: classification.errorType, consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount })
      } else {
        this.persist(contState, { status: "Error", consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount })
      }
      if (recoveryDecision.action === "Backoff") this.lastDispatchTime = Date.now() - this.config.minContinuationIntervalMs + recoveryDecision.delayMs
      await this.logDecision("error", "Lux continuation dispatch failed", { reason, error: classification.message, errorType: classification.errorType, recoveryAction: recoveryDecision.action, delayMs: recoveryDecision.delayMs })
      return this.result(false, null, ticket.id ?? null, message)
    }
  }
}

export const LuxPlugin = async (ctx: LuxPluginContext) => {
  const { project, client, directory, worktree } = ctx
  const projectPath = directory ?? worktree ?? process.cwd()
  const log = async (level: "debug" | "info" | "warn" | "error", message: string, extra?: Record<string, unknown>) => {
    await client?.app?.log?.({ body: { service: SERVICE, level, message, extra } })
  }

  const toast = async (message: string, variant: "success" | "error" | "info" = "info") => {
    await client?.tui?.showToast?.({ body: { message, variant } })
  }

  await log("info", "Lux plugin loaded", { project, directory, worktree })

  const gatewayUrl = process.env?.LUX_GATEWAY_URL || "http://localhost:18766"
  let poller: ReturnType<typeof createProgressPoller> | null = null
  let currentSessionID = "lux-session"
  const state = stateFor(projectPath)
  const integrator = createExternalSignalIntegrator(projectPath)
  await ensureGatewayRunning({
    gatewayUrl,
    projectPath,
    healthTimeoutMs: Number(process.env?.LUX_GATEWAY_STARTUP_TIMEOUT_MS) || 15000,
  })
  void ctx.client?.tui?.showToast?.({ body: _buildToastMessage("info", ["✅ Lux Autonomous Driving loaded", `Gateway: ${gatewayUrl}`]) })
  const orchestrator = new ContinuationOrchestrator({
    config: { projectPath, gatewayUrl },
    deps: { stateClient: continuationStateClient, ticketLoader, signalIntegrator: () => integrator },
    ctx,
    state,
    getSessionID: () => currentSessionID,
    logDecision: log,
  })
  const sessionEndHandlers: Array<(event: Record<string, unknown>) => void> = []
  const sessionEndDetector = setupSessionEndDetection({
    registerEvent(handler) {
      sessionEndHandlers.push(handler)
      return () => {
        const index = sessionEndHandlers.indexOf(handler)
        if (index >= 0) sessionEndHandlers.splice(index, 1)
      }
    },
    projectPath,
    getProcessing: () => orchestrator.isProcessing(),
  }, async () => {
    await orchestrator.onTrigger("session-end-detector")
  })
  void sessionEndDetector

  function startPoller() {
    if (poller) return

    poller = createProgressPoller({
      gatewayUrl,
      projectPath,
      pollIntervalMs: Number(process.env?.LUX_POLL_INTERVAL_MS) || 5000,
      onProgress(diff) {
        if (diff.changedTickets.length > 0) {
          void orchestrator.onTrigger("progress-change")
        }
      },
      onError(err) {
        console.warn("[Lux] Progress poller error:", err.message)
      },
    })

    poller.start()
  }

  function stopPoller() {
    if (poller) {
      poller.stop()
      poller = null
    }
  }

  startPoller()

  return {
    event: async ({ event }: { event: OpenCodeEvent }) => {
      const type = valueToString(event.type)
      const sessionID = resolveSessionID(event)
      currentSessionID = sessionID
      for (const handler of sessionEndHandlers) handler(eventProperties(event))

      if (type === "session.idle") {
        if (poller) poller.poll().catch(() => {})
        await orchestrator.onTrigger("session.idle")
        const compileResult = await checkAndFixCompile(ctx, { projectPath, gatewayUrl, sessionID }, state)
        if (compileResult.hasErrors && !compileResult.wasFixed) {
          void ctx.client?.tui?.showToast?.({ body: _buildToastMessage("error", [
            `⚠️ Compile errors persist (${compileResult.errors.length})`,
            "Retries exhausted — manual fix needed",
          ]) })
        } else if (!compileResult.hasErrors && state.consecutiveCompileFailures > 0) {
          state.consecutiveCompileFailures = 0
          void ctx.client?.tui?.showToast?.({ body: _buildToastMessage("info", [
            "✅ All compile errors fixed",
            `${compileResult.warnings.length} warnings remain`,
          ]) })
        }
        return
      }

      if (type === "session.compacted") {
        armCompactionGuard(state)
        await log("info", "Lux armed compaction guard", { sessionID, epoch: state.recentCompactionEpoch })
        return
      }

      if (type === "session.status") {
        const properties = eventProperties(event)
        const status = valueToString(properties.status)
        const message = valueToString(properties.message || properties.error)
        const lowerStatus = status.toLowerCase()
        const lowerMessage = message.toLowerCase()

        await log("info", "Lux observed session status", { status, message, event: properties })

        if (lowerStatus === "end" || lowerStatus === "shutdown") {
          stopPoller()
          await log("info", "Lux stopped progress poller", { reason: `session.${status}` })
        }

        if (lowerStatus === "error" || lowerMessage.includes("error")) {
          state.consecutiveFailures += 1
          await toast(`Lux: ${message || "Session status error"}`, "error")
        }
        if (lowerStatus === "cancelled" || lowerMessage.includes("cancel")) {
          state.abortDetectedAt = Date.now()
        }
      }
    },

    "tool.execute.after": async (input: ToolInput, output: unknown) => {
      const toolName = valueToString(input.tool)
      if (!toolName.startsWith("lux_")) return

      const status = resultStatus(output)
      const variant = status.toLowerCase() === "error" ? "error" : "info"

      await log("info", "Lux tool completed", { tool: toolName, status })
      await toast(`Lux: ${toolName} ${status}`, variant)

      const state = stateFor(projectPath)
      const evalResult = evaluateSpec(projectPath)
      const tickets = loadTickets(projectPath)
      integrator.reportToolExecution({ tool: toolName, success: status.toLowerCase() !== "error" })
      trackProgress(state, tickets.tickets, evalResult.ambiguity_score, integrator)
      if (MUTATING_LUX_TOOLS.has(toolName)) {
        invalidateCache()
        await orchestrator.onTrigger(`tool.execute.after:${toolName}`)
      }
    },

    "experimental.session.compacting": async (_input: unknown, output: CompactingOutput) => {
      const state = stateFor(projectPath)
      armCompactionGuard(state)
      state.acknowledgedCompactionEpoch = state.recentCompactionEpoch

      const tickets = loadTickets(projectPath)
      const context = [
        _formatContextBlock({ byStatus: tickets.byStatus, activeTicketsCount: tickets.activeTickets.length, incompleteCount: tickets.incompleteCount, totalTickets: tickets.tickets.length }),
        `Continuation: ${state.continuationCount}/${MAX_CONTINUATIONS}`,
        state.consecutiveFailures > 0 ? `⚠️ Failures: ${state.consecutiveFailures}` : "",
      ].filter(Boolean).join("\n")

      if (!Array.isArray(output.context)) output.context = []
      output.context.push(context)

      await log("debug", "Lux context injected during session compacting")
    },
  }
}
