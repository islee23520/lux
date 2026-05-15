import { acknowledgeCompaction, armCompactionGuard, isCompactionGuardActive } from "./compaction-guard"
import { checkAndFixCompile } from "./compile-guard"
import { readContinuationState, writeContinuationState, type ContinuationState, type ContinuationStateWriteOptions, type ContinuationWriteResult } from "./continuation-state-client"
import { createExternalSignalIntegrator, type ExternalSignalIntegrator } from "./external-signal-integrator"
import { classifyError, getBackoffDelayMs, handlePromptAsyncError, type RecoveryState } from "./error-recovery"
import { ensureGatewayRunning } from "./gateway-spawn"
import { generateNextAction } from "./next-action-generator"
import { buildToastMessage, formatContextBlock, formatStatus } from "./lux-overlay"
import { buildContinuationPrompt } from "./prompt-builder"
import { createProgressPoller } from "./progress-poller"
import { evaluateSpec } from "./spec-evaluator"
import { getState, type LuxSessionState } from "./session-state"
import { setupSessionEndDetection } from "./session-end-detector"
import { getStagnationDetails, shouldStopForStagnation, trackProgress } from "./stagnation-detection"
import { evaluateStopConditions, type StopDecision } from "./stop-evaluator"
import { invalidateCache, loadTickets, type Ticket } from "./ticket-loader"
import type { LuxPluginConfig } from "./types"

declare const process: { env: Record<string, string | undefined> }

interface OpenCodePluginEvent {
  type: string
  properties?: Record<string, unknown>
}

interface OpenCodePluginEventPayload {
  event: OpenCodePluginEvent
}

interface OpenCodePluginServerContext {
  directory: string
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
  }
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
  stateClient: {
    readContinuationState: (opts: { gatewayUrl: string; projectPath: string }) => Promise<ContinuationState>
    writeContinuationState: (opts: ContinuationStateWriteOptions, state: ContinuationState) => Promise<ContinuationWriteResult>
  }
  ticketLoader: {
    loadTickets: typeof loadTickets
    invalidateCache: typeof invalidateCache
  }
  signalIntegrator: () => ExternalSignalIntegrator | undefined
}

export type StopReason =
  | "max_continuations_reached"
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

interface OpenCodePluginServerResult {
  tool: Record<string, never>
  event: (payload: OpenCodePluginEventPayload) => Promise<void>
  "tool.execute.after": (input: { tool?: string; [key: string]: unknown }, output: unknown) => Promise<void>
  "experimental.session.compacting": (input: unknown, output: { context?: string[]; [key: string]: unknown }) => Promise<void>
}

interface OpenCodePlugin {
  id: string
  server: (ctx: OpenCodePluginServerContext) => Promise<OpenCodePluginServerResult>
}

const DEFAULT_CONFIG: LuxPluginConfig = {
  maxContinuations: 50,
  specPath: ".lux/spec.json",
  glossaryPath: ".lux/glossary.md",
  targetAmbiguity: 0.02,
}

const MIN_CONTINUATION_INTERVAL_MS = 3000
const MAX_CONSECUTIVE_FAILURES = 3
const MAX_STAGNATION = 3

const MUTATING_LUX_TOOLS = new Set([
  "lux_init",
  "lux_spec_apply",
  "lux_spec_update",
  "lux_ticket_create",
  "lux_ticket_update",
  "lux_ticket_status",
  "lux_verify",
])

function valueToString(value: unknown): string {
  return typeof value === "string" ? value : value === undefined || value === null ? "" : String(value)
}

function resolveSessionID(event: OpenCodePluginEvent): string {
  const props = event.properties ?? {}
  const candidates = [props.sessionID, props.sessionId, props.id]
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.length > 0) return candidate
  }
  return "lux-session"
}

function priorityRank(priority: string | undefined): number {
  const ranks: Record<string, number> = { Critical: 0, High: 1, Medium: 2, Low: 3 }
  return ranks[priority ?? ""] ?? 99
}

function stringProp(ticket: Ticket, key: string): string | undefined {
  const value = ticket[key]
  return typeof value === "string" ? value : undefined
}

function blockerCount(ticket: Ticket): number {
  const blockers = ticket.blockers
  return Array.isArray(blockers) ? blockers.length : 0
}

function selectBestTicket(tickets: Ticket[]): Ticket | null {
  const active = tickets.filter((ticket) => ticket.status !== "Done" && ticket.status !== "Blocked")
  if (active.length === 0) return null

  const inProgress = active.filter((ticket) => ticket.status === "InProgress")
  if (inProgress.length > 0) {
    return inProgress.sort((a, b) => priorityRank(stringProp(a, "priority")) - priorityRank(stringProp(b, "priority")))[0] ?? null
  }

  const todo = active.filter((ticket) => ticket.status === "ToDo" || ticket.status === "Todo")
  return todo
    .sort((a, b) => {
      const pDiff = priorityRank(stringProp(a, "priority")) - priorityRank(stringProp(b, "priority"))
      if (pDiff !== 0) return pDiff
      // Blocker count only breaks ties here; this is a sort, so circular dependencies cannot recurse.
      return blockerCount(a) - blockerCount(b)
    })[0] ?? null
}

function continuationStateWithStatus(state: ContinuationState, patch: Partial<ContinuationState> & { status?: ContinuationState["status"] | "Complete" }): ContinuationState {
  return { ...state, ...patch } as ContinuationState
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
  private readonly ctx: OpenCodePluginServerContext
  private readonly state: LuxSessionState
  private readonly getSessionID: () => string
  private readonly recoveryState: RecoveryState = createRecoveryState()
  private lastDispatchTime = 0
  private lastKnownSeq = 0

  constructor(args: {
    config: OrchestratorConfig
    deps: OrchestratorDeps
    ctx: OpenCodePluginServerContext
    state: LuxSessionState
    getSessionID: () => string
  }) {
    this.config = {
      projectPath: args.config.projectPath,
      gatewayUrl: args.config.gatewayUrl,
      maxContinuations: args.config.maxContinuations ?? DEFAULT_CONFIG.maxContinuations,
      minContinuationIntervalMs: args.config.minContinuationIntervalMs ?? MIN_CONTINUATION_INTERVAL_MS,
      healthThreshold: args.config.healthThreshold ?? 20,
      maxStagnation: args.config.maxStagnation ?? MAX_STAGNATION,
    }
    this.deps = args.deps
    this.ctx = args.ctx
    this.state = args.state
    this.getSessionID = args.getSessionID
  }

  isProcessing(): boolean {
    return this.state.inFlight
  }

  private result(dispatched: boolean, stopReason: StopReason, selectedTicketId: string | null, message: string): CycleResult {
    return { dispatched, stopReason, selectedTicketId, message }
  }

  private async persist(contState: ContinuationState, patch: Partial<ContinuationState> & { status?: ContinuationState["status"] | "Complete" }, expectedSeq: number): Promise<ContinuationState> {
    const next = continuationStateWithStatus(contState, patch)
    const opts: ContinuationStateWriteOptions = {
      gatewayUrl: this.config.gatewayUrl,
      projectPath: this.config.projectPath,
      expectedSeq,
    }
    const result = await this.deps.stateClient.writeContinuationState(opts, next)
    this.lastKnownSeq = result.seq
    return next
  }

  async onTrigger(reason: string): Promise<CycleResult> {
    const now = Date.now()
    const backoff = this.recoveryState.lastErrorType
      ? getBackoffDelayMs(this.recoveryState.lastErrorType, this.recoveryState.consecutiveErrorsByType[this.recoveryState.lastErrorType])
      : 0
    const interval = Math.max(this.config.minContinuationIntervalMs, backoff)
    if (now - this.lastDispatchTime < interval) return this.result(false, null, null, "rate_limited")

    let contState = await this.deps.stateClient.readContinuationState({ gatewayUrl: this.config.gatewayUrl, projectPath: this.config.projectPath })
    this.state.continuationCount = contState.continuation_count
    this.state.stagnationCount = contState.stagnation_count
    this.state.consecutiveFailures = contState.consecutive_failures
    if (contState.status === "Stopped") return this.result(false, contState.stop_reason as StopReason, contState.current_ticket_id, "stopped")

    const ticketSummary = this.deps.ticketLoader.loadTickets(this.config.projectPath)
    const evalResult = evaluateSpec(this.config.projectPath, DEFAULT_CONFIG)
    const ambiguity = evalResult.ambiguity_score
    const integrator = this.deps.signalIntegrator()
    trackProgress(this.state, ticketSummary.tickets, ambiguity, integrator)
    const healthScore = integrator?.getHealthScore()
    const stagnation = getStagnationDetails(this.state, integrator, this.config.maxStagnation)
    const recoveryAction = integrator?.getNextActionSuggestion()
    const hasRecoveryAction = Boolean(recoveryAction && !recoveryAction.toLowerCase().includes("continue with next ticket"))
    const activeTickets = ticketSummary.tickets.filter((ticket) => ticket.status !== "Done" && ticket.status !== "Blocked")
    const hasClarificationInProgress = ticketSummary.tickets.some((ticket) => ticket.status === "InProgress" && (ticket.title ?? "").toLowerCase().includes("clarify"))
    const stopDecision: StopDecision = evaluateStopConditions({
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
      const stopLevel = stopDecision.reason === "all_complete" ? ("info" as const) : ("warn" as const)
        const stopToast = buildToastMessage(stopLevel, [
          `⛔ Lux stopped: ${stopDecision.reason}`,
          formatStatus(
          { status: contState.status, continuationCount: this.state.continuationCount, consecutiveFailures: this.state.consecutiveFailures, currentTicketId: contState.current_ticket_id, stopReason: stopDecision.reason },
          { byStatus: ticketSummary.byStatus, activeTicketsCount: ticketSummary.activeTickets.length, incompleteCount: ticketSummary.incompleteCount },
        ),
      ])
      void this.ctx.client?.tui?.showToast?.({ body: stopToast })
      if (stopDecision.reason === "all_complete") {
        contState = await this.persist(contState, { status: "Complete", stop_reason: "all_complete" }, this.lastKnownSeq)
        return this.result(false, "all_complete", null, "all_complete")
      }
      if (stopDecision.reason === "consecutive_state_error") {
        return this.result(false, stopDecision.reason, contState.current_ticket_id, "continuation_state_error")
      }
      if (stopDecision.reason === "stagnation") {
        this.lastDispatchTime = now - this.config.minContinuationIntervalMs + getBackoffDelayMs("unknown", Math.max(1, stagnation.reasons.length))
      }
      contState = await this.persist(contState, { status: "Stopped", stop_reason: stopDecision.reason, stagnation_count: this.state.stagnationCount, consecutive_failures: this.state.consecutiveFailures }, this.lastKnownSeq)
      return this.result(false, stopDecision.reason, contState.current_ticket_id, stopDecision.reason)
    }
    if (isCompactionGuardActive(this.state)) return this.result(false, null, contState.current_ticket_id, "compaction_guard")
    if (this.state.inFlight || this.state.consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) return this.result(false, null, contState.current_ticket_id, "blocked")

    const ticket = selectBestTicket(ticketSummary.tickets)
    if (!ticket) return this.result(false, null, null, "no_ticket")

    const nextAction = generateNextAction({
      activeTickets: [ticket, ...activeTickets.filter((item) => item !== ticket)],
      inactiveTickets: ticketSummary.tickets.filter((item) => item.status === "Done" || item.status === "Blocked"),
      ticketCounts: ticketSummary.byStatus,
      incompleteCount: ticketSummary.incompleteCount,
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
      await this.persist(contState, { status: "Error", consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount }, this.lastKnownSeq)
      return this.result(false, null, ticket.id ?? null, "promptAsync unavailable")
    }

    const message = buildContinuationPrompt({
      ticket,
      nextAction,
      ambiguity,
      summary: ticketSummary,
      continuationCount: this.state.continuationCount,
      consecutiveFailures: this.state.consecutiveFailures,
    })
    this.state.inFlight = true
    try {
      await promptAsync({
        path: { id: this.getSessionID() },
        body: { parts: [{ type: "text", text: message }] },
        query: { directory: this.ctx.directory },
      })
      this.state.inFlight = false
      this.lastDispatchTime = Date.now()
      this.state.lastInjectedAt = this.lastDispatchTime
      this.state.awaitingPostInjectionProgressCheck = true
      this.state.consecutiveFailures = 0
      this.state.continuationCount += 1
      contState = await this.persist(contState, {
        session_id: this.getSessionID(),
        continuation_count: this.state.continuationCount,
        stagnation_count: this.state.stagnationCount,
        consecutive_failures: this.state.consecutiveFailures,
        last_ambiguity: String(ambiguity),
        last_ticket_baseline: String(ticketSummary.incompleteCount),
        current_ticket_id: ticket.id ?? contState.current_ticket_id,
        status: "Active",
        stop_reason: null,
        started_at: contState.started_at ?? new Date().toISOString(),
      }, this.lastKnownSeq)
      this.deps.ticketLoader.invalidateCache()
      console.debug("[Lux] Continuation decision", {
        projectPath: this.config.projectPath,
        reason,
        dispatched: true,
        selectedTicketId: ticket.id ?? null,
        nextActionReason: nextAction.reason,
        healthScore,
        ambiguityScore: ambiguity,
        activeTicketCount: activeTickets.length,
        incompleteTicketCount: ticketSummary.incompleteCount,
        continuationCount: this.state.continuationCount,
        stagnationCount: this.state.stagnationCount,
      })
      const dispatchToast = buildToastMessage("info", [
        formatStatus(
          { status: contState.status, continuationCount: this.state.continuationCount, consecutiveFailures: this.state.consecutiveFailures, currentTicketId: ticket.id ?? null },
          { byStatus: ticketSummary.byStatus, activeTicketsCount: ticketSummary.activeTickets.length, incompleteCount: ticketSummary.incompleteCount, totalTickets: ticketSummary.tickets.length },
          { dispatched: true, reason, selectedTicketId: ticket.id ?? null, healthScore, ambiguityScore: ambiguity, activeTicketCount: activeTickets.length, incompleteTicketCount: ticketSummary.incompleteCount, stagnationCount: this.state.stagnationCount, continuationCount: this.state.continuationCount },
        ),
      ])
      void this.ctx.client?.tui?.showToast?.({ body: dispatchToast })
      return this.result(true, null, ticket.id ?? null, message)
    } catch (err) {
      this.state.inFlight = false
      this.state.lastInjectedAt = Date.now()
      this.state.consecutiveFailures += 1
      const classification = classifyError(err)
      const recoveryDecision = handlePromptAsyncError(err, this.recoveryState)
      if (recoveryDecision.action === "Abort") {
        await this.persist(contState, { status: "Stopped", stop_reason: classification.errorType, consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount }, this.lastKnownSeq)
      } else {
        await this.persist(contState, { status: "Error", consecutive_failures: this.state.consecutiveFailures, stagnation_count: this.state.stagnationCount }, this.lastKnownSeq)
      }
      if (recoveryDecision.action === "Backoff") this.lastDispatchTime = Date.now() - this.config.minContinuationIntervalMs + recoveryDecision.delayMs
      console.error("[Lux] Continuation dispatch failed", { reason, error: classification.message, errorType: classification.errorType, recoveryAction: recoveryDecision.action, delayMs: recoveryDecision.delayMs })
      return this.result(false, null, ticket.id ?? null, message)
    }
  }
}

const plugin = {
  id: "lux-spec-orchestrator",
  server: async (ctx: OpenCodePluginServerContext) => {
    const config = DEFAULT_CONFIG
    const projectPath = ctx.directory
    const gatewayUrl = process.env.LUX_GATEWAY_URL || "http://localhost:18766"
    let poller: ReturnType<typeof createProgressPoller> | null = null
    let currentSessionID = "lux-session"
    const state = getState(projectPath)
    const integrator = createExternalSignalIntegrator(projectPath)
    await ensureGatewayRunning({
      gatewayUrl,
      projectPath,
      healthTimeoutMs: Number(process.env.LUX_GATEWAY_STARTUP_TIMEOUT_MS) || 15000,
    })
    void ctx.client?.tui?.showToast?.({ body: buildToastMessage("info", [`✅ Lux Autonomous Driving loaded`, `Gateway: ${gatewayUrl}`]) })
    const orchestrator = new ContinuationOrchestrator({
      config: { projectPath, gatewayUrl, maxContinuations: config.maxContinuations },
      deps: {
        stateClient: { readContinuationState, writeContinuationState },
        ticketLoader: { loadTickets, invalidateCache },
        signalIntegrator: () => integrator,
      },
      ctx,
      state,
      getSessionID: () => currentSessionID,
    })
    const sessionEndHandlers: Array<(event: Record<string, unknown>) => void> = []
    setupSessionEndDetection({
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

    function startPoller(): void {
      if (poller) return

      poller = createProgressPoller({
        gatewayUrl,
        projectPath,
        pollIntervalMs: Number(process.env.LUX_POLL_INTERVAL_MS) || 5000,
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

    function stopPoller(): void {
      if (poller) {
        poller.stop()
        poller = null
      }
    }

    startPoller()

    return {
      tool: {},
      event: async ({ event }: OpenCodePluginEventPayload) => {
        const sessionID = resolveSessionID(event)
        currentSessionID = sessionID
        const state = getState(ctx.directory)
        for (const handler of sessionEndHandlers) handler({ ...event.properties, type: event.type })

        if (event.type === "session.idle") {
          if (poller) poller.poll().catch(() => {})
          await orchestrator.onTrigger("session.idle")
          const compileResult = await checkAndFixCompile(ctx, { projectPath, gatewayUrl, sessionID }, state)
          if (compileResult.hasErrors && !compileResult.wasFixed) {
            void ctx.client?.tui?.showToast?.({ body: buildToastMessage("error", [
              `⚠️ Compile errors persist (${compileResult.errors.length})`,
              "Retries exhausted — manual fix needed",
            ]) })
          } else if (!compileResult.hasErrors && state.consecutiveCompileFailures > 0) {
            state.consecutiveCompileFailures = 0
            void ctx.client?.tui?.showToast?.({ body: buildToastMessage("info", [
              "✅ All compile errors fixed",
              `${compileResult.warnings.length} warnings remain`,
            ]) })
          }
          return
        }

        if (event.type === "session.compacted") {
          armCompactionGuard(state)
          return
        }

        if (event.type === "session.status") {
          const status = valueToString(event.properties?.status).toLowerCase()
          const message = valueToString(event.properties?.message ?? event.properties?.error).toLowerCase()
        if (status === "end" || status === "shutdown") {
          stopPoller()
          console.debug("[Lux] Stopped progress poller", { reason: `session.${status}` })
        }
          if (status === "error" || message.includes("error")) state.consecutiveFailures += 1
          if (status === "cancelled" || message.includes("cancel")) state.abortDetectedAt = Date.now()
        }
      },
      "tool.execute.after": async (input: { tool?: string; [key: string]: unknown }, output: unknown) => {
        const toolName = valueToString(input.tool)
        if (!toolName.startsWith("lux_")) return

        const state = getState(ctx.directory)
        const evalResult = evaluateSpec(ctx.directory, config)
        const ticketSummary = loadTickets(ctx.directory)
        const outputRecord = typeof output === "object" && output !== null ? output as Record<string, unknown> : {}
        const failed = outputRecord.error !== undefined || outputRecord.ok === false || outputRecord.status === "error"
        integrator.reportToolExecution({ tool: toolName, success: !failed })
        trackProgress(state, ticketSummary.tickets, evalResult.ambiguity_score, integrator)

        if (MUTATING_LUX_TOOLS.has(toolName)) {
          invalidateCache()
          await orchestrator.onTrigger(`tool.execute.after:${toolName}`)
        }
      },
      "experimental.session.compacting": async (_input: unknown, output: { context?: string[]; [key: string]: unknown }) => {
        const state = getState(ctx.directory)
        armCompactionGuard(state)
        acknowledgeCompaction(state, state.recentCompactionEpoch)

        const ticketSummary = loadTickets(ctx.directory)
        const context = [
          formatContextBlock({ byStatus: ticketSummary.byStatus, activeTicketsCount: ticketSummary.activeTickets.length, incompleteCount: ticketSummary.incompleteCount, totalTickets: ticketSummary.tickets.length }),
          `Continuation: ${state.continuationCount}/${config.maxContinuations}`,
          state.consecutiveFailures > 0 ? `⚠️ Failures: ${state.consecutiveFailures}` : "",
        ].filter(Boolean).join("\n")

        if (!Array.isArray(output.context)) output.context = []
        output.context.push(context)
      },
    }
  },
} satisfies OpenCodePlugin

export default plugin
