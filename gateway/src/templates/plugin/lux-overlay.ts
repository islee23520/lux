// lux-overlay.ts — Smart Toast & Context formatter for Lux autonomous driving status overlay
// Pure formatter module — no side effects, no I/O beyond what caller provides.

export type OverlayLevel = "info" | "warn" | "error" | "silent"

const MAX_TOAST_LENGTH = 500
const MAX_CONTEXT_LENGTH = 800

// ─── Internal helpers ────────────────────────────────────

function statusIcon(status?: string): string {
  if (!status) return "◦️"
  switch (status) {
    case "Active": return "✅"
    case "Complete": return "🏁"
    case "Stopped": return "⛔"
    case "Error": return "💥"
    case "Idle": return "⏸️"
    default: return "◔️"
  }
}

function healthIcon(score: number | undefined): string {
  if (score === undefined) return "—"
  if (score >= 70) return "💚"
  if (score >= 40) return "🟡"
  return "🔴"
}

function truncate(str: string, max: number): string {
  if (str.length <= max) return str
  return str.slice(0, max - 1) + "…"
}

function present<T>(value: T | undefined, fallback: T): T {
  return value ?? fallback
}

// ─── Public API ──────────────────────────────────────

export interface StatusInput {
  status?: string
  continuationCount?: number
  consecutiveFailures?: number
  currentTicketId?: string | null
  stopReason?: string | null
}

export interface SummaryInput {
  byStatus?: Record<string, number>
  activeTicketsCount?: number
  incompleteCount?: number
  totalTickets?: number
}

export interface DecisionInput {
  dispatched?: boolean
  reason?: string
  selectedTicketId?: string | null
  healthScore?: number | undefined
  ambiguityScore?: number
  activeTicketCount?: number
  incompleteTicketCount?: number
  stagnationCount?: number
  continuationCount?: number
}

/**
 * Format a multi-line status block for toast display.
 * Uses box-drawing chars for visual structure.
 * Truncates to MAX_TOAST_LENGTH.
 */
export function formatStatus(
  state?: StatusInput,
  summary?: SummaryInput,
  decision?: DecisionInput,
): string {
  const icon = statusIcon(state?.status)
  const count = state?.continuationCount ?? "?"
  const failures = present(state?.consecutiveFailures, 0)
  const ticket = state?.currentTicketId ?? "—"

  // Progress line
  let progress = ""
  if (summary) {
    const done = present(summary.byStatus?.Done, 0)
    const incompleteCount = summary.incompleteCount ?? 0
    const total = summary.totalTickets ?? incompleteCount + done
    const active = present(summary.activeTicketsCount, 0)
    if (total > 0 || done > 0 || active > 0) {
      progress = `📋 ${done}/${total} done  ⚡${active} active`
    }
  }

  // Health line
  let health = ""
  if (decision?.healthScore !== undefined) {
    health = `${healthIcon(decision.healthScore)} ${decision.healthScore}`
  }

  // Ambiguity line
  let amb = ""
  if (decision?.ambiguityScore !== undefined) {
    amb = `🔋 amb:${decision.ambiguityScore}`
  }

  // Next action line
  let next = ""
  if (decision?.reason && decision.dispatched) {
    next = `→ ${present(decision.selectedTicketId, "idle")}`
  } else if (decision?.reason) {
    next = `⏸ ${decision.reason}`
  }

  // Stop reason line (only show if stopped/error/complete)
  let stop = ""
  if (state?.status === "Complete" || state?.status === "Stopped" || state?.status === "Error") {
    stop = `${icon} ${state.status}`
    if (state?.stopReason) stop += ` (${state.stopReason})`
  }

  const lines = [
    `┌─ Lux Autonomous ─────────────────────┐`,
    `│ ${icon} ${state?.status ?? "Unknown"}  ${count}  ${progress}   ${health} ${amb} ${failures > 0 ? `⚠️${failures}` : ""}`,
    next ? `│ ${next}` : "",
    ticket ? `│ 🎯 ${ticket}` : "",
    stop ? `│ ${stop}` : "",
    `└───────────────────────────────────┘`,
  ]

  return truncate(lines.filter(Boolean).join("\n"), MAX_TOAST_LENGTH)
}

/**
 * Format compact ASCII status block for AI compaction context.
 * Must be ≤ MAX_CONTEXT_LENGTH.
 */
export function formatContextBlock(summary?: SummaryInput): string {
  if (!summary) return "[Lux: no data]"

  const done = present(summary.byStatus?.Done, 0)
  const todo = present(summary.byStatus?.ToDo ?? present(summary.byStatus?.Todo, 0), 0)
  const ip = present(summary.activeTicketsCount, 0)
  const incompleteCount = summary.incompleteCount ?? 0
  const total = summary.totalTickets ?? incompleteCount + done
  const incomplete = present(summary.incompleteCount, total - done)

  const parts = [
    `[Lux Status]`,
    `Tickets: ${done} Done / ${todo} ToDo / ${ip} InProgress (${total} total)`,
    incomplete > 0 ? `Remaining: ${incomplete}` : "",
  ].filter(Boolean)

  return truncate(parts.join("  "), MAX_CONTEXT_LENGTH)
}

/**
 * Map OverlayLevel to OpenCode showToast variant.
 * Join sections with newlines, truncate to max length.
 */
export function buildToastMessage(
  level: OverlayLevel,
  sections: string[],
): { variant: "success" | "error" | "info"; message: string } {
  const variantMap: Record<OverlayLevel, "success" | "error" | "info"> = {
    info: "success",
    warn: "error",
    error: "error",
    silent: "info",
  }
  const variant = variantMap[level] ?? "info"
  const message = truncate(sections.filter(Boolean).join("\n"), MAX_TOAST_LENGTH)
  return { variant, message }
}
