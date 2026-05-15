import { useCallback, useEffect, useRef, useState } from 'react'

const TOKEN = 'dev-token'

export interface DomainAmbiguityDatum {
  domain: string
  ambiguity: number
  completion: number
  status: string
  requirements_total: number
  requirements_done: number
}

export interface KanbanStatusDatum {
  status: string
  count: number
}

export interface ProgressSummaryData {
  spec: {
    overall_ambiguity: number
    domains: Record<string, {
      ambiguity: number
      status: string
      requirements_total: number
      requirements_done: number
    }>
  }
  kanban: {
    by_status: Record<string, number>
    total: number
    active_count: number
  }
  loop: {
    state: string
    iteration: number | null
  }
}

export interface UseSpecKanbanProgressResult {
  data: ProgressSummaryData | null
  loading: boolean
  error: string | null
  wsConnected: boolean
  refresh: () => Promise<void>
}

interface EventEnvelope {
  category?: string
  payload?: {
    kind?: string
    [key: string]: unknown
  }
}

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

function isSpecKanbanProgressEvent(envelope: EventEnvelope): boolean {
  const kind = envelope.payload?.kind?.toLowerCase() ?? ''
  const category = envelope.category?.toLowerCase() ?? ''
  return ['spec:progress', 'kanban:progress'].some((needle) => {
    return kind.includes(needle) || category.includes(needle)
  })
}

async function fetchProgressSummary(projectPath: string): Promise<ProgressSummaryData> {
  return request<ProgressSummaryData>(withProject('/api/lux/progress/summary', projectPath))
}

export function useSpecKanbanProgress(projectPath: string | null | undefined): UseSpecKanbanProgressResult {
  const [data, setData] = useState<ProgressSummaryData | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [wsConnected, setWsConnected] = useState(false)
  const refreshTimeoutRef = useRef<number | null>(null)

  const refresh = useCallback(async () => {
    if (!projectPath) {
      setData(null)
      setLoading(false)
      setError('Unity project path is required for progress summary')
      return
    }

    try {
      setLoading(true)
      const nextData = await fetchProgressSummary(projectPath)
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
    
    // Fallback polling every 10s
    const interval = window.setInterval(() => void refresh(), 10000)
    return () => window.clearInterval(interval)
  }, [refresh])

  useEffect(() => {
    if (!projectPath) return undefined

    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    const clientId = `spec-kanban-${Math.random().toString(16).slice(2)}`
    const ws = new WebSocket(
      `${protocol}//${window.location.host}/events?token=${encodeURIComponent(TOKEN)}&role=subscriber&client_id=${encodeURIComponent(clientId)}`,
    )

    ws.onopen = () => setWsConnected(true)
    ws.onclose = () => setWsConnected(false)
    ws.onerror = () => setWsConnected(false)
    ws.onmessage = (message) => {
      try {
        const envelope = JSON.parse(String(message.data)) as EventEnvelope
        if (isSpecKanbanProgressEvent(envelope)) {
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
