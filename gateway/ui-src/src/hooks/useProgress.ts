import { useCallback, useEffect, useRef, useState } from 'react'

const TOKEN = 'dev-token'

export interface DomainAmbiguityDatum {
  domain: string
  ambiguity: number
  completion: number
}

export interface KanbanStatusDatum {
  status: TicketStatus
  count: number
}

export interface BuildHistoryDatum {
  id: string
  status: BuildStatusKind
  label: string
  startedAt: string | null
  completedAt: string | null
  progress: number
  error: string | null
}

export interface PlayEventTypeDatum {
  eventType: string
  count: number
}

export interface PlaySessionStatsData {
  sessionCount: number
  totalEvents: number
  totalDurationSecs: number
  averageDurationSecs: number
  eventTypes: PlayEventTypeDatum[]
  recentSessions: SessionMetadata[]
}

export interface ProgressData {
  specCompletion: DomainAmbiguityDatum[]
  overallAmbiguity: number
  kanbanStatus: KanbanStatusDatum[]
  buildHistory: BuildHistoryDatum[]
  playSessionStats: PlaySessionStatsData
}

export interface UseProgressResult {
  data: ProgressData | null
  loading: boolean
  error: string | null
  wsConnected: boolean
  refresh: () => Promise<void>
}

interface LuxAmbiguityResponse {
  overall: number
  domains: Record<string, number>
}

type TicketStatus = 'Backlog' | 'Blocked' | 'ToDo' | 'InProgress' | 'Done'

interface Ticket {
  id: string
  status: TicketStatus
}

type RawBuildStatus = 'Queued' | 'Running' | 'Succeeded' | 'Cancelled' | { Failed: string }
type BuildStatusKind = 'Queued' | 'Running' | 'Succeeded' | 'Failed' | 'Cancelled'

interface BuildJob {
  build_id: string
  status: RawBuildStatus
  progress: number
  started_at: string | null
  completed_at: string | null
  error: string | null
}

export interface SessionMetadata {
  session_id: string
  started_at: string
  ended_at: string | null
  duration_secs: number | null
  event_count: number
  webgl_build_version: string | null
  player_id: string | null
}

interface PlayEvent {
  event_type: string | { Custom: string }
}

interface EventEnvelope {
  category?: string
  payload?: {
    kind?: string
    [key: string]: unknown
  }
}

const ticketStatuses: TicketStatus[] = ['Backlog', 'Blocked', 'ToDo', 'InProgress', 'Done']

async function request<T>(path: string): Promise<T> {
  const response = await fetch(path, {
    headers: {
      'content-type': 'application/json',
      'x-lux-token': TOKEN,
    },
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`GET ${path} failed: ${response.status} ${body}`)
  }

  return (await response.json()) as T
}

function withProject(path: string, projectPath: string): string {
  const separator = path.includes('?') ? '&' : '?'
  return `${path}${separator}project_path=${encodeURIComponent(projectPath)}`
}

function clamp01(value: number): number {
  if (!Number.isFinite(value)) return 0
  return Math.min(1, Math.max(0, value))
}

function normalizeBuildStatus(status: RawBuildStatus): { kind: BuildStatusKind; error: string | null } {
  if (typeof status === 'string') {
    return { kind: status, error: null }
  }
  return { kind: 'Failed', error: status.Failed }
}

function normalizeEventType(eventType: PlayEvent['event_type']): string {
  return typeof eventType === 'string' ? eventType : eventType.Custom
}

function sortByTimestampDescending<T>(items: T[], getTimestamp: (item: T) => string | null): T[] {
  return [...items].sort((left, right) => {
    const leftTime = Date.parse(getTimestamp(left) ?? '') || 0
    const rightTime = Date.parse(getTimestamp(right) ?? '') || 0
    return rightTime - leftTime
  })
}

function isProgressEvent(envelope: EventEnvelope): boolean {
  const kind = envelope.payload?.kind?.toLowerCase() ?? ''
  const category = envelope.category?.toLowerCase() ?? ''
  return ['lux', 'spec', 'kanban', 'ticket', 'build', 'play', 'session'].some((needle) => {
    return kind.includes(needle) || category.includes(needle)
  })
}

async function fetchBuildJobs(): Promise<BuildJob[]> {
  try {
    return await request<BuildJob[]>('/api/lux/build/jobs')
  } catch (error) {
    if (error instanceof Error && error.message.includes('404')) {
      return request<BuildJob[]>('/api/lux/build/list')
    }
    throw error
  }
}

async function fetchProgress(projectPath: string): Promise<ProgressData> {
  const [ambiguity, tickets, builds, sessions] = await Promise.all([
    request<LuxAmbiguityResponse>(withProject('/api/lux/spec/ambiguity', projectPath)),
    request<Ticket[]>(withProject('/api/lux/kanban/tickets', projectPath)),
    fetchBuildJobs(),
    request<SessionMetadata[]>(withProject('/api/lux/play/sessions', projectPath)),
  ])

  const specCompletion = Object.entries(ambiguity.domains)
    .map(([domain, score]) => {
      const normalized = clamp01(score)
      return {
        domain,
        ambiguity: normalized,
        completion: 1 - normalized,
      }
    })
    .sort((left, right) => right.ambiguity - left.ambiguity)

  const kanbanStatus = ticketStatuses.map((status) => ({
    status,
    count: tickets.filter((ticket) => ticket.status === status).length,
  }))

  const buildHistory = sortByTimestampDescending(builds, (build) => build.completed_at ?? build.started_at)
    .slice(0, 20)
    .reverse()
    .map((build) => {
      const status = normalizeBuildStatus(build.status)
      return {
        id: build.build_id,
        status: status.kind,
        label: build.build_id.slice(0, 8),
        startedAt: build.started_at,
        completedAt: build.completed_at,
        progress: clamp01(build.progress),
        error: build.error ?? status.error,
      }
    })

  const recentSessions = sortByTimestampDescending(sessions, (session) => session.started_at).slice(0, 10)
  const eventsBySession = await Promise.all(
    recentSessions.map((session) => {
      const path = withProject(`/api/lux/play/sessions/${encodeURIComponent(session.session_id)}/events?limit=500`, projectPath)
      return request<PlayEvent[]>(path)
    }),
  )
  const eventTypeCounts = new Map<string, number>()
  for (const event of eventsBySession.flat()) {
    const eventType = normalizeEventType(event.event_type)
    eventTypeCounts.set(eventType, (eventTypeCounts.get(eventType) ?? 0) + 1)
  }

  const totalDurationSecs = sessions.reduce((sum, session) => sum + (session.duration_secs ?? 0), 0)
  const completedSessionCount = sessions.filter((session) => session.duration_secs !== null).length

  return {
    specCompletion,
    overallAmbiguity: clamp01(ambiguity.overall),
    kanbanStatus,
    buildHistory,
    playSessionStats: {
      sessionCount: sessions.length,
      totalEvents: sessions.reduce((sum, session) => sum + session.event_count, 0),
      totalDurationSecs,
      averageDurationSecs: completedSessionCount > 0 ? totalDurationSecs / completedSessionCount : 0,
      eventTypes: Array.from(eventTypeCounts.entries())
        .map(([eventType, count]) => ({ eventType, count }))
        .sort((left, right) => right.count - left.count),
      recentSessions,
    },
  }
}

export function useProgress(projectPath: string | null | undefined): UseProgressResult {
  const [data, setData] = useState<ProgressData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [wsConnected, setWsConnected] = useState(false)
  const refreshTimeoutRef = useRef<number | null>(null)

  const refresh = useCallback(async () => {
    if (!projectPath) {
      setData(null)
      setLoading(false)
      setError('Unity project path is required for progress graphs')
      return
    }

    try {
      setLoading(true)
      const nextData = await fetchProgress(projectPath)
      setData(nextData)
      setError(null)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }, [projectPath])

  const scheduleRefresh = useCallback(() => {
    if (refreshTimeoutRef.current !== null) {
      window.clearTimeout(refreshTimeoutRef.current)
    }
    refreshTimeoutRef.current = window.setTimeout(() => {
      refreshTimeoutRef.current = null
      void refresh()
    }, 250)
  }, [refresh])

  useEffect(() => {
    void refresh()
  }, [refresh])

  useEffect(() => {
    if (!projectPath) return undefined

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const clientId = `progress-${Math.random().toString(16).slice(2)}`
    const ws = new WebSocket(
      `${protocol}//${window.location.host}/events?token=${encodeURIComponent(TOKEN)}&role=subscriber&client_id=${encodeURIComponent(clientId)}`,
    )

    ws.onopen = () => setWsConnected(true)
    ws.onclose = () => setWsConnected(false)
    ws.onerror = () => setWsConnected(false)
    ws.onmessage = (message) => {
      try {
        const envelope = JSON.parse(String(message.data)) as EventEnvelope
        if (isProgressEvent(envelope)) {
          scheduleRefresh()
        }
      } catch {
        scheduleRefresh()
      }
    }

    return () => {
      ws.close()
      setWsConnected(false)
      if (refreshTimeoutRef.current !== null) {
        window.clearTimeout(refreshTimeoutRef.current)
        refreshTimeoutRef.current = null
      }
    }
  }, [projectPath, scheduleRefresh])

  return { data, loading, error, wsConnected, refresh }
}
