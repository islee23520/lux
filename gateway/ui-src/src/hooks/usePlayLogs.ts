import { useCallback, useEffect, useMemo, useState } from 'react'

const TOKEN = 'dev-token'

export const PLAY_LOG_FILTER_TYPES = [
  'PlayerAction',
  'SystemEvent',
  'ErrorEvent',
  'FeedbackEvent',
  'StateChange',
  'Custom',
] as const

export type PlayLogFilterType = (typeof PLAY_LOG_FILTER_TYPES)[number]

export type PlayEventType =
  | 'Action'
  | 'Decision'
  | 'Trigger'
  | 'Death'
  | 'LevelComplete'
  | 'LevelStart'
  | 'ItemCollect'
  | 'Damage'
  | 'MenuOpen'
  | 'MenuClose'
  | 'CutsceneStart'
  | 'CutsceneEnd'
  | 'Save'
  | 'Load'
  | PlayLogFilterType
  | { Custom: string }

export type JsonValue = string | number | boolean | null | JsonValue[] | { [key: string]: JsonValue }

export interface PlayEvent {
  session_id: string
  timestamp: string
  event_type: PlayEventType
  payload: JsonValue
  player_id: string | null
  game_state: JsonValue | null
  sequence: number
}

export interface SessionMetadata {
  session_id: string
  started_at: string
  ended_at: string | null
  duration_secs: number | null
  event_count: number
  webgl_build_version: string | null
  player_id: string | null
  metadata?: Record<string, JsonValue>
}

export interface PlayLogFilters {
  eventTypes: PlayLogFilterType[]
  from: string
  to: string
}

export interface PlayLogStatistics {
  totalEvents: number
  totalSessions: number
  averageSessionDurationSecs: number
  countsByType: Record<PlayLogFilterType, number>
}

export interface UsePlayLogsResult {
  sessions: SessionMetadata[]
  selectedSessionId: string | null
  events: PlayEvent[]
  filteredEvents: PlayEvent[]
  filters: PlayLogFilters
  statistics: PlayLogStatistics
  loadingSessions: boolean
  loadingEvents: boolean
  error: string | null
  selectSession: (sessionId: string | null) => void
  setFilters: (filters: PlayLogFilters) => void
  refresh: () => Promise<void>
}

const emptyCounts = (): Record<PlayLogFilterType, number> => ({
  PlayerAction: 0,
  SystemEvent: 0,
  ErrorEvent: 0,
  FeedbackEvent: 0,
  StateChange: 0,
  Custom: 0,
})

const defaultFilters: PlayLogFilters = {
  eventTypes: [...PLAY_LOG_FILTER_TYPES],
  from: '',
  to: '',
}

const isRecord = (value: JsonValue | undefined): value is { [key: string]: JsonValue } => {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export const normalizePlayEventType = (eventType: PlayEventType): string => {
  return typeof eventType === 'string' ? eventType : eventType.Custom
}

const textFromValue = (value: JsonValue | undefined): string => {
  if (typeof value === 'string') return value
  if (typeof value === 'number' || typeof value === 'boolean') return String(value)
  return ''
}

export const eventFilterTypeForEvent = (event: PlayEvent): PlayLogFilterType => {
  const rawType = normalizePlayEventType(event.event_type)
  if (PLAY_LOG_FILTER_TYPES.some((type) => type === rawType)) {
    return rawType as PlayLogFilterType
  }

  const payload = isRecord(event.payload) ? event.payload : undefined
  const kind = textFromValue(payload?.kind).toLowerCase()
  const category = textFromValue(payload?.category).toLowerCase()
  const message = `${textFromValue(payload?.message)} ${textFromValue(payload?.description)}`.toLowerCase()
  const rawTypeLower = rawType.toLowerCase()
  const searchable = `${rawTypeLower} ${kind} ${category} ${message}`

  if (searchable.includes('feedback')) return 'FeedbackEvent'
  if (['Death', 'Damage'].includes(rawType) || searchable.includes('error') || searchable.includes('exception')) {
    return 'ErrorEvent'
  }
  if (
    ['LevelStart', 'LevelComplete', 'Save', 'Load', 'MenuOpen', 'MenuClose', 'CutsceneStart', 'CutsceneEnd'].includes(rawType) ||
    searchable.includes('state')
  ) {
    return 'StateChange'
  }
  if (['Action', 'Decision', 'Trigger', 'ItemCollect'].includes(rawType) || searchable.includes('player')) {
    return 'PlayerAction'
  }
  if (typeof event.event_type !== 'string') return 'Custom'
  return 'SystemEvent'
}

export const describePlayEvent = (event: PlayEvent): string => {
  const payload = isRecord(event.payload) ? event.payload : undefined
  const explicit = textFromValue(payload?.description) || textFromValue(payload?.message) || textFromValue(payload?.action)
  if (explicit) return explicit
  return `${normalizePlayEventType(event.event_type)} #${event.sequence}`
}

const parseApiError = async (response: Response): Promise<string> => {
  const body = await response.text()
  return body.trim() ? body : response.statusText
}

class ApiRequestError extends Error {
  readonly status: number

  constructor(status: number, message: string) {
    super(message)
    this.status = status
  }
}

const fetchJson = async <T>(path: string): Promise<T> => {
  const response = await fetch(path, {
    headers: {
      'content-type': 'application/json',
      'x-lux-token': TOKEN,
    },
  })

  if (!response.ok) {
    throw new ApiRequestError(response.status, await parseApiError(response))
  }

  return (await response.json()) as T
}

const buildQuery = (params: Record<string, string | number | undefined>): string => {
  const search = new URLSearchParams()
  for (const [key, value] of Object.entries(params)) {
    if (value !== undefined && String(value).trim() !== '') {
      search.set(key, String(value))
    }
  }
  return search.toString()
}

const toIsoFromDateTimeLocal = (value: string): string | undefined => {
  if (!value) return undefined
  const date = new Date(value)
  return Number.isNaN(date.getTime()) ? undefined : date.toISOString()
}

const eventMatchesFilters = (event: PlayEvent, filters: PlayLogFilters): boolean => {
  const eventType = eventFilterTypeForEvent(event)
  if (!filters.eventTypes.includes(eventType)) return false

  const timestamp = Date.parse(event.timestamp)
  if (filters.from) {
    const from = Date.parse(filters.from)
    if (!Number.isNaN(from) && timestamp < from) return false
  }
  if (filters.to) {
    const to = Date.parse(filters.to)
    if (!Number.isNaN(to) && timestamp > to) return false
  }
  return true
}

const sortEvents = (events: PlayEvent[]): PlayEvent[] => {
  return [...events].sort((left, right) => {
    const leftTime = Date.parse(left.timestamp) || 0
    const rightTime = Date.parse(right.timestamp) || 0
    if (leftTime !== rightTime) return leftTime - rightTime
    return left.sequence - right.sequence
  })
}

export function usePlayLogs(projectPath: string | null | undefined): UsePlayLogsResult {
  const [sessions, setSessions] = useState<SessionMetadata[]>([])
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null)
  const [events, setEvents] = useState<PlayEvent[]>([])
  const [filters, setFilters] = useState<PlayLogFilters>(defaultFilters)
  const [loadingSessions, setLoadingSessions] = useState(false)
  const [loadingEvents, setLoadingEvents] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadSessions = useCallback(async (): Promise<SessionMetadata[]> => {
    if (!projectPath) return []
    setLoadingSessions(true)
    try {
      const query = buildQuery({ project_path: projectPath })
      const loaded = await fetchJson<SessionMetadata[]>(`/api/lux/play/sessions?${query}`)
      const sorted = [...loaded].sort((left, right) => (Date.parse(right.started_at) || 0) - (Date.parse(left.started_at) || 0))
      setSessions(sorted)
      setSelectedSessionId((current) => current ?? sorted[0]?.session_id ?? null)
      return sorted
    } finally {
      setLoadingSessions(false)
    }
  }, [projectPath])

  const loadEvents = useCallback(async (): Promise<void> => {
    if (!projectPath) {
      setEvents([])
      return
    }

    setLoadingEvents(true)
    try {
      const fromTime = toIsoFromDateTimeLocal(filters.from)
      const toTime = toIsoFromDateTimeLocal(filters.to)
      const baseParams = { project_path: projectPath, from_time: fromTime, to_time: toTime, limit: 1000 }
      let loaded: PlayEvent[]

      if (selectedSessionId) {
        const query = buildQuery(baseParams)
        loaded = await fetchJson<PlayEvent[]>(`/api/lux/play/sessions/${encodeURIComponent(selectedSessionId)}/events?${query}`)
      } else {
        const type = filters.eventTypes.length === 1 ? filters.eventTypes[0] : undefined
        const query = buildQuery({ project_path: projectPath, type, from: fromTime, to: toTime, limit: 1000 })
        try {
          loaded = await fetchJson<PlayEvent[]>(`/api/lux/play/events?${query}`)
        } catch (caught) {
          if (!(caught instanceof ApiRequestError) || (caught.status !== 404 && caught.status !== 405)) {
            throw caught
          }
          const sessionEvents = await Promise.all(
            sessions.map((session) => {
              const sessionQuery = buildQuery(baseParams)
              return fetchJson<PlayEvent[]>(`/api/lux/play/sessions/${encodeURIComponent(session.session_id)}/events?${sessionQuery}`)
            }),
          )
          loaded = sessionEvents.flat()
        }
      }

      setEvents(sortEvents(loaded))
    } finally {
      setLoadingEvents(false)
    }
  }, [filters.eventTypes, filters.from, filters.to, projectPath, selectedSessionId])

  const refresh = useCallback(async (): Promise<void> => {
    setError(null)
    try {
      await loadSessions()
      await loadEvents()
    } catch (caught) {
      setError(caught instanceof Error ? caught.message : 'Failed to load play logs')
    }
  }, [loadEvents, loadSessions])

  useEffect(() => {
    void loadSessions().catch((caught) => {
      setError(caught instanceof Error ? caught.message : 'Failed to load play sessions')
    })
  }, [loadSessions])

  useEffect(() => {
    void loadEvents().catch((caught) => {
      setError(caught instanceof Error ? caught.message : 'Failed to load play events')
    })
  }, [loadEvents])

  const filteredEvents = useMemo(() => {
    return events.filter((event) => eventMatchesFilters(event, filters))
  }, [events, filters])

  const statistics = useMemo<PlayLogStatistics>(() => {
    const countsByType = emptyCounts()
    for (const event of filteredEvents) {
      const type = eventFilterTypeForEvent(event)
      countsByType[type] += 1
    }

    const completedDurations = sessions
      .map((session) => session.duration_secs)
      .filter((duration): duration is number => typeof duration === 'number' && Number.isFinite(duration))
    const totalDuration = completedDurations.reduce((sum, duration) => sum + duration, 0)

    return {
      totalEvents: filteredEvents.length,
      totalSessions: sessions.length,
      averageSessionDurationSecs: completedDurations.length > 0 ? totalDuration / completedDurations.length : 0,
      countsByType,
    }
  }, [filteredEvents, sessions])

  return {
    sessions,
    selectedSessionId,
    events,
    filteredEvents,
    filters,
    statistics,
    loadingSessions,
    loadingEvents,
    error,
    selectSession: setSelectedSessionId,
    setFilters,
    refresh,
  }
}
