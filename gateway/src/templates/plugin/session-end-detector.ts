import type { ContinuationState } from "./continuation-state-client"
import type { LuxSessionState } from "./session-state"

export interface SessionEndContext {
  projectPath: string
  activeTickets: Array<{ id: string; title: string; status: string }>
  totalTickets: number
  incompleteCount: number
  lastState: ContinuationState
  detectedAt: number
  reason: "session.end" | "session.shutdown" | "session.error" | "manual"
}

export type SessionEndCallback = (ctx: SessionEndContext) => Promise<void> | void

export interface SessionEndDetector {
  /** Stop listening for session end events */
  destroy(): void
  /** Manually trigger a resume check */
  triggerManualResume(): void
}

export type SessionEndEventHandler = (event: Record<string, unknown>) => void
export type SessionEndEventRegistrar = (handler: SessionEndEventHandler) => void | (() => void)

export interface SessionEndDetectionOptions {
  registerEvent: SessionEndEventRegistrar
  projectPath?: string
  getProcessing?: () => boolean
}

export interface SessionEndDetectorControls extends SessionEndDetector {
  /** Flush a queued resume when no promptAsync call is in flight. */
  consumePendingResume(): void
  /** Mirror continuation-injector promptAsync in-flight state. */
  setProcessing(value: boolean): void
  /** Update the project path used when events do not carry one. */
  setProjectPath(projectPath: string): void
}

let activeListener: (() => void) | null = null
let pendingResume = false
let isProcessing = false

function defaultContinuationState(): ContinuationState {
  return {
    session_id: null,
    continuation_count: 0,
    stagnation_count: 0,
    consecutive_failures: 0,
    last_ambiguity: null,
    last_ticket_baseline: null,
    current_ticket_id: null,
    status: "Idle",
    started_at: null,
    updated_at: "",
    stop_reason: null,
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function valueToString(value: unknown): string {
  return typeof value === "string" ? value : value === undefined || value === null ? "" : String(value)
}

function normalizeStatus(value: string): string {
  return value.trim().toLowerCase().replace(/[\s_-]/g, "")
}

function resolveStatus(eventData: Record<string, unknown>): "end" | "shutdown" | "error" | null {
  const properties = isRecord(eventData.properties) ? eventData.properties : {}
  const type = valueToString(eventData.type).toLowerCase()
  const status = valueToString(eventData.status ?? properties.status).toLowerCase()

  const candidates = [status, type.startsWith("session.") ? type.slice("session.".length) : type]
  for (const candidate of candidates) {
    if (candidate === "end" || candidate === "shutdown" || candidate === "error") return candidate
  }

  const message = valueToString(eventData.message ?? properties.message ?? properties.error).toLowerCase()
  return message.includes("error") ? "error" : null
}

function resolveReason(status: "end" | "shutdown" | "error"): SessionEndContext["reason"] {
  return status === "error" ? "session.error" : `session.${status}`
}

function resolveProjectPath(eventData: Record<string, unknown>, fallback: string): string {
  const properties = isRecord(eventData.properties) ? eventData.properties : {}
  const candidates = [eventData.projectPath, eventData.directory, eventData.cwd, properties.projectPath, properties.directory, properties.cwd]
  for (const candidate of candidates) {
    if (typeof candidate === "string" && candidate.length > 0) return candidate
  }
  return fallback
}

function summarizeActiveTickets(tickets: Array<{ id?: string; title?: string; status: string }>): SessionEndContext["activeTickets"] {
  return tickets
    .filter((ticket) => {
      const status = normalizeStatus(ticket.status)
      return status === "todo" || status === "inprogress"
    })
    .map((ticket) => ({
      id: ticket.id ?? "",
      title: ticket.title ?? "Untitled ticket",
      status: ticket.status,
    }))
}

async function buildSessionEndContext(
  projectPath: string,
  reason: SessionEndContext["reason"],
): Promise<{ ctx: SessionEndContext; abortDetectedAt: number | undefined }> {
  if (projectPath.length === 0) {
    return {
      ctx: {
        projectPath,
        activeTickets: [],
        totalTickets: 0,
        incompleteCount: 0,
        lastState: defaultContinuationState(),
        detectedAt: Date.now(),
        reason,
      },
      abortDetectedAt: undefined,
    }
  }

  const [{ readContinuationState }, { loadTickets }, { getState }] = await Promise.all([
    import("./continuation-state-client"),
    import("./ticket-loader"),
    import("./session-state"),
  ])
  const ticketSummary = loadTickets(projectPath)
  const sessionState: LuxSessionState = getState(projectPath)

  return {
    ctx: {
      projectPath,
      activeTickets: summarizeActiveTickets(ticketSummary.tickets),
      totalTickets: ticketSummary.tickets.length,
      incompleteCount: ticketSummary.incompleteCount,
      lastState: await readContinuationState({ gatewayUrl: process.env.LUX_GATEWAY_URL || "http://localhost:18766", projectPath }),
      detectedAt: Date.now(),
      reason,
    },
    abortDetectedAt: sessionState.abortDetectedAt,
  }
}

export function shouldAutoResume(
  incompleteTicketCount: number,
  stopReason: string | null | undefined,
  abortDetectedAt: number | undefined,
): boolean {
  if (abortDetectedAt) return false
  if (stopReason === "user_abort") return false
  return incompleteTicketCount > 0
}

export function formatSessionEndSummary(ctx: SessionEndContext): string {
  const lines = [
    `[Lux] Session ended: ${ctx.reason}`,
    `Active tickets remaining: ${ctx.activeTickets.length}/${ctx.totalTickets}`,
    `Incomplete: ${ctx.incompleteCount}`,
  ]

  if (ctx.activeTickets.length > 0) {
    lines.push("Remaining:")
    for (const ticket of ctx.activeTickets.slice(0, 5)) {
      lines.push(`  [${ticket.status}] ${ticket.title} (${ticket.id})`)
    }
  }

  return lines.join("\n")
}

export function manualResume(ctx: SessionEndContext, onSessionEnd?: SessionEndCallback): SessionEndContext {
  const resumeCtx: SessionEndContext = {
    ...ctx,
    detectedAt: Date.now(),
    reason: "manual",
  }

  if (onSessionEnd && shouldAutoResume(resumeCtx.incompleteCount, resumeCtx.lastState.stop_reason, undefined)) {
    void onSessionEnd(resumeCtx)
  }

  return resumeCtx
}

function normalizeSetupOptions(input: SessionEndEventRegistrar | SessionEndDetectionOptions): SessionEndDetectionOptions {
  if (typeof input === "function") return { registerEvent: input }
  return input
}

export function setupSessionEndDetection(
  registerEvent: SessionEndEventRegistrar,
  onSessionEnd: SessionEndCallback,
): SessionEndDetectorControls
export function setupSessionEndDetection(
  options: SessionEndDetectionOptions,
  onSessionEnd: SessionEndCallback,
): SessionEndDetectorControls
export function setupSessionEndDetection(
  input: SessionEndEventRegistrar | SessionEndDetectionOptions,
  onSessionEnd: SessionEndCallback,
): SessionEndDetectorControls {
  const options = normalizeSetupOptions(input)
  let projectPath = options.projectPath ?? ""
  let lastQueuedReason: SessionEndContext["reason"] = "manual"
  let destroyed = false

  function processingActive(): boolean {
    return isProcessing || Boolean(options.getProcessing?.())
  }

  async function dispatch(project: string, reason: SessionEndContext["reason"]): Promise<void> {
    const { ctx, abortDetectedAt } = await buildSessionEndContext(project, reason)
    if (!shouldAutoResume(ctx.incompleteCount, ctx.lastState.stop_reason, abortDetectedAt)) {
      console.debug(formatSessionEndSummary(ctx))
      return
    }

    console.debug(formatSessionEndSummary(ctx))
    await onSessionEnd(ctx)
  }

  function queueResume(reason: SessionEndContext["reason"]): void {
    pendingResume = true
    lastQueuedReason = reason
    console.warn("[Lux] Session end detected during processing, queuing resume")
  }

  function handleSessionEnd(eventData: Record<string, unknown>): void {
    const status = resolveStatus(eventData)
    if (!status || destroyed) return

    const reason = resolveReason(status)
    const eventProjectPath = resolveProjectPath(eventData, projectPath)
    if (eventProjectPath.length > 0) projectPath = eventProjectPath

    if (processingActive()) {
      queueResume(reason)
      return
    }

    void dispatch(projectPath, reason)
  }

  const cleanup = options.registerEvent(handleSessionEnd)
  activeListener = typeof cleanup === "function" ? cleanup : null

  const detector: SessionEndDetectorControls = {
    destroy() {
      destroyed = true
      if (activeListener) activeListener()
      activeListener = null
      pendingResume = false
    },
    triggerManualResume() {
      if (processingActive()) {
        queueResume("manual")
        return
      }
      void dispatch(projectPath, "manual")
    },
    consumePendingResume() {
      if (!pendingResume || processingActive() || destroyed) return
      pendingResume = false
      void dispatch(projectPath, lastQueuedReason)
    },
    setProcessing(value: boolean) {
      isProcessing = value
      if (!value) this.consumePendingResume()
    },
    setProjectPath(nextProjectPath: string) {
      projectPath = nextProjectPath
    },
  }

  return detector
}
