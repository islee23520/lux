export interface SpecProgress {
  overall_ambiguity?: number
  domains?: Record<string, DomainProgress>
  [key: string]: unknown
}

export interface DomainProgress {
  ambiguity?: number
  status?: string
  requirements_total?: number
  requirements_done?: number
  [key: string]: unknown
}

export interface KanbanProgress {
  tickets?: ProgressTicket[]
  activeTickets?: ProgressTicket[]
  active_tickets?: ProgressTicket[]
  byStatus?: Record<string, number>
  by_status?: Record<string, number>
  total?: number
  active_count?: number
  [key: string]: unknown
}

export interface LoopProgress {
  state?: string
  iteration?: number | null
  [key: string]: unknown
}

export interface ProgressTicket {
  id?: string
  title?: string
  status?: string
  [key: string]: unknown
}

export interface ProgressSummary {
  spec?: SpecProgress
  kanban?: KanbanProgress
  loop_summary?: LoopProgress
  loop?: LoopProgress
}

export interface ProgressDiff {
  previousSummary: ProgressSummary | null
  currentSummary: ProgressSummary
  changedTickets: TicketChange[]
  progressDelta: Record<string, number>
  timestamp: number
}

export interface TicketChange {
  id: string
  title: string
  previousStatus: string
  newStatus: string
}

export interface ProgressPollerConfig {
  gatewayUrl: string
  projectPath: string
  pollIntervalMs?: number
  onProgress: (diff: ProgressDiff) => void
  onError?: (error: Error) => void
}

export interface ProgressPoller {
  start: () => void
  stop: () => void
  poll: () => Promise<void>
}

const MAX_RETRIES = 3
const BASE_BACKOFF_MS = 1000
const MAX_BACKOFF_INTERVAL_MS = 60_000

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function asError(value: unknown): Error {
  return value instanceof Error ? value : new Error(String(value))
}

function delay(ms: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve) => {
    if (signal.aborted) {
      resolve()
      return
    }

    const handle = setTimeout(resolve, ms)
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(handle)
        resolve()
      },
      { once: true },
    )
  })
}

function normalizeGatewayUrl(gatewayUrl: string): string {
  return gatewayUrl.replace(/\/+$/, "")
}

function ticketKey(ticket: ProgressTicket): string | null {
  return typeof ticket.id === "string" && ticket.id.length > 0 ? ticket.id : null
}

function ticketStatus(ticket: ProgressTicket): string {
  return typeof ticket.status === "string" && ticket.status.length > 0 ? ticket.status : "Unknown"
}

function ticketTitle(ticket: ProgressTicket): string {
  return typeof ticket.title === "string" ? ticket.title : ""
}

function ticketArray(value: unknown): ProgressTicket[] {
  if (!Array.isArray(value)) return []
  return value.filter((item): item is ProgressTicket => isRecord(item))
}

function ticketsFromSummary(summary: ProgressSummary): ProgressTicket[] {
  const kanban = summary.kanban
  if (!kanban) return []

  const directTickets = ticketArray(kanban.tickets)
  if (directTickets.length > 0) return directTickets

  const activeTickets = ticketArray(kanban.activeTickets)
  if (activeTickets.length > 0) return activeTickets

  return ticketArray(kanban.active_tickets)
}

function statusCounts(summary: ProgressSummary): Record<string, number> {
  const kanban = summary.kanban
  if (!kanban) return {}

  if (isRecord(kanban.by_status)) return normalizeCounts(kanban.by_status)
  if (isRecord(kanban.byStatus)) return normalizeCounts(kanban.byStatus)

  const counts: Record<string, number> = {}
  for (const ticket of ticketsFromSummary(summary)) {
    const status = ticketStatus(ticket)
    counts[status] = (counts[status] ?? 0) + 1
  }
  return counts
}

function normalizeCounts(counts: Record<string, unknown>): Record<string, number> {
  const normalized: Record<string, number> = {}
  for (const [status, count] of Object.entries(counts)) {
    if (typeof count === "number" && Number.isFinite(count)) {
      normalized[status] = count
    }
  }
  return normalized
}

function computeTicketChanges(
  previousSummary: ProgressSummary | null,
  currentSummary: ProgressSummary,
): TicketChange[] {
  if (!previousSummary) return []

  const previousTickets = new Map<string, ProgressTicket>()
  for (const ticket of ticketsFromSummary(previousSummary)) {
    const id = ticketKey(ticket)
    if (id) previousTickets.set(id, ticket)
  }

  const changes: TicketChange[] = []
  for (const currentTicket of ticketsFromSummary(currentSummary)) {
    const id = ticketKey(currentTicket)
    if (!id) continue

    const previousTicket = previousTickets.get(id)
    if (!previousTicket) continue

    const previousStatus = ticketStatus(previousTicket)
    const newStatus = ticketStatus(currentTicket)
    if (previousStatus === newStatus) continue

    changes.push({
      id,
      title: ticketTitle(currentTicket) || ticketTitle(previousTicket),
      previousStatus,
      newStatus,
    })
  }

  return changes
}

function computeProgressDelta(
  previousSummary: ProgressSummary | null,
  currentSummary: ProgressSummary,
): Record<string, number> {
  const previousCounts = previousSummary ? statusCounts(previousSummary) : {}
  const currentCounts = statusCounts(currentSummary)
  const statuses = new Set([...Object.keys(previousCounts), ...Object.keys(currentCounts)])
  const delta: Record<string, number> = {}

  for (const status of statuses) {
    const change = (currentCounts[status] ?? 0) - (previousCounts[status] ?? 0)
    if (change !== 0) delta[status] = change
  }

  return delta
}

function hasMeaningfulChange(diff: ProgressDiff): boolean {
  return diff.previousSummary === null || diff.changedTickets.length > 0 || Object.keys(diff.progressDelta).length > 0
}

export function createProgressPoller(config: ProgressPollerConfig): ProgressPoller {
  const {
    gatewayUrl,
    projectPath,
    pollIntervalMs = 5000,
    onProgress,
    onError = () => {},
  } = config

  let previousSummary: ProgressSummary | null = null
  let timerHandle: ReturnType<typeof setTimeout> | null = null
  let abortController = new AbortController()
  let consecutiveErrors = 0
  let running = false
  let inFlight = false

  function buildUrl(): string {
    return `${normalizeGatewayUrl(gatewayUrl)}/api/lux/progress/summary?project_path=${encodeURIComponent(projectPath)}`
  }

  async function fetchSummary(): Promise<ProgressSummary> {
    try {
      const response = await fetch(buildUrl(), { signal: abortController.signal })
      if (!response.ok) throw new Error(`HTTP ${response.status}`)
      return response.json() as Promise<ProgressSummary>
    } catch (error) {
      // Network failures are retried by the caller so a temporary gateway outage does not stop polling.
      throw asError(error)
    }
  }

  function computeDiff(current: ProgressSummary): ProgressDiff {
    return {
      previousSummary,
      currentSummary: current,
      changedTickets: computeTicketChanges(previousSummary, current),
      progressDelta: computeProgressDelta(previousSummary, current),
      timestamp: Date.now(),
    }
  }

  function nextInterval(): number {
    if (consecutiveErrors <= MAX_RETRIES) return pollIntervalMs

    const failedSchedules = consecutiveErrors - MAX_RETRIES
    const multiplier = 2 ** Math.max(1, failedSchedules)
    return Math.min(pollIntervalMs * multiplier, MAX_BACKOFF_INTERVAL_MS)
  }

  function clearTimer(): void {
    if (timerHandle) {
      clearTimeout(timerHandle)
      timerHandle = null
    }
  }

  function scheduleNext(): void {
    clearTimer()
    if (!running || abortController.signal.aborted) return

    timerHandle = setTimeout(() => {
      void poll()
    }, nextInterval())
  }

  async function poll(): Promise<void> {
    try {
      if (abortController.signal.aborted || inFlight) return

      inFlight = true
      let lastError: Error | null = null

      for (let attempt = 0; attempt <= MAX_RETRIES; attempt += 1) {
        try {
          const current = await fetchSummary()
          const diff = computeDiff(current)
          previousSummary = current
          consecutiveErrors = 0

          if (hasMeaningfulChange(diff)) onProgress(diff)
          return
        } catch (error) {
          lastError = asError(error)
          if (abortController.signal.aborted) return

          consecutiveErrors += 1
          onError(lastError)

          if (attempt < MAX_RETRIES) {
            await delay(BASE_BACKOFF_MS * 2 ** attempt, abortController.signal)
          }
        }
      }

      if (lastError) onError(lastError)
    } catch (error) {
      if (!abortController.signal.aborted) onError(asError(error))
    } finally {
      inFlight = false
      scheduleNext()
    }
  }

  function start(): void {
    if (running) return

    if (abortController.signal.aborted) {
      abortController = new AbortController()
    }

    running = true
    void poll()
  }

  function stop(): void {
    running = false
    clearTimer()
    abortController.abort()
  }

  return { start, stop, poll }
}
