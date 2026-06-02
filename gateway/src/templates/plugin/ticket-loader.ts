import * as fs from "node:fs"
import * as path from "node:path"

export type TicketStatus = "Backlog" | "Blocked" | "ToDo" | "InProgress" | "Done" | string

export interface Ticket {
  id?: string
  title?: string
  description?: string
  status: TicketStatus
  type?: string
  spec_ref?: string | null
  tags?: string[]
  [key: string]: unknown
}

export interface TicketSummary {
  tickets: Ticket[]
  byStatus: Record<string, number>
  activeTickets: Ticket[]
  incompleteCount: number
}

let cachedSummary: TicketSummary | null = null
let cachedAt = 0
let cachedProjectPath = ""
let cacheTtlMs = 10_000

const STATUS_MAP: Record<string, TicketStatus> = {
  backlog: "Backlog",
  blocked: "Blocked",
  todo: "ToDo",
  todo_: "ToDo",
  inprogress: "InProgress",
  in_progress: "InProgress",
  done: "Done",
}

export function invalidateCache(): void {
  cachedSummary = null
  cachedAt = 0
  cachedProjectPath = ""
}

export function setCacheTtl(ms: number): void {
  cacheTtlMs = ms
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null
}

function normalizeStatus(raw: string): TicketStatus {
  const lower = raw.trim().toLowerCase()
  const compact = lower.replace(/[\s_-]/g, "")
  return STATUS_MAP[lower] ?? STATUS_MAP[compact] ?? raw
}

function normalizeTicket(value: unknown): Ticket | null {
  if (!isRecord(value)) return null

  const status = typeof value.status === "string" && value.status.length > 0 ? normalizeStatus(value.status) : "Backlog"
  const tags = Array.isArray(value.tags) ? value.tags.filter((tag): tag is string => typeof tag === "string") : []
  const specRef = typeof value.spec_ref === "string" && value.spec_ref.length > 0 ? value.spec_ref : null

  return {
    ...(value as Record<string, unknown>),
    status,
    spec_ref: specRef,
    tags,
  } satisfies Ticket
}

export function countByStatus(tickets: Ticket[]): Record<string, number> {
  const byStatus: Record<string, number> = {}
  for (const ticket of tickets) {
    byStatus[ticket.status] = (byStatus[ticket.status] ?? 0) + 1
  }
  return byStatus
}

export function getActiveTickets(tickets: Ticket[]): Ticket[] {
  return tickets.filter((ticket) => (ticket.status === "ToDo" || ticket.status === "InProgress") && Boolean(ticket.spec_ref))
}

export function loadTickets(projectPath: string): TicketSummary {
  const now = Date.now()
  if (cachedSummary && cachedProjectPath === projectPath && now - cachedAt < cacheTtlMs) {
    return cachedSummary
  }

  const ticketsDir = path.join(projectPath, ".lux", "tickets")
  let entries: string[]

  try {
    entries = fs.readdirSync(ticketsDir)
  } catch { /* intentional: missing ticket directory falls back to empty set */
    entries = []
  }

  const tickets: Ticket[] = []
  for (const entry of entries) {
    if (!entry.endsWith(".json")) continue

    try {
      const content = fs.readFileSync(path.join(ticketsDir, entry), "utf-8")
      const ticket = normalizeTicket(JSON.parse(content))
      if (ticket) tickets.push(ticket)
    } catch (err) {
      console.warn(`[lux-ticket-loader] Skipping malformed ticket file "${entry}":`, err)
      continue
    }
  }

  const byStatus = countByStatus(tickets)
  const activeTickets = getActiveTickets(tickets)
  const incompleteCount = tickets.filter((ticket) => ticket.status !== "Done").length

  const result = { tickets, byStatus, activeTickets, incompleteCount }
  cachedSummary = result
  cachedProjectPath = projectPath
  cachedAt = now
  return result
}

export function getTicketById(projectPath: string, id: string): Ticket | null {
  const { tickets } = loadTickets(projectPath)
  return tickets.find((ticket) => ticket.id === id) ?? null
}

export { normalizeStatus, STATUS_MAP }
