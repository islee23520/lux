import { useCallback, useEffect, useMemo, useRef, useState } from 'react'

export type TerminalStatus = 'active' | 'closed'
export type TerminalStream = 'stdout' | 'stderr'

export interface TerminalOutput {
  sessionId: string
  data: string
  timestamp: string
  stream: TerminalStream
}

export interface TerminalSession {
  sessionId: string
  createdAt: string
  status: TerminalStatus
  outputBuffer: TerminalOutput[]
  history: string[]
}

interface RawTerminalOutput {
  session_id?: string
  sessionId?: string
  data: string
  timestamp: string
  stream: TerminalStream
}

interface RawTerminalSession {
  session_id?: string
  sessionId?: string
  created_at?: string
  createdAt?: string
  status: TerminalStatus
  output_buffer?: RawTerminalOutput[]
  outputBuffer?: RawTerminalOutput[]
  history?: string[]
}

type TerminalWsMessage = {
  type?: string
  session_id?: string
  sessionId?: string
  data?: string
  payload?: Record<string, unknown>
}

const normalizeOutput = (output: RawTerminalOutput): TerminalOutput => ({
  sessionId: output.session_id ?? output.sessionId ?? '',
  data: output.data,
  timestamp: output.timestamp,
  stream: output.stream,
})

const normalizeSession = (session: RawTerminalSession): TerminalSession => ({
  sessionId: session.session_id ?? session.sessionId ?? '',
  createdAt: session.created_at ?? session.createdAt ?? '',
  status: session.status,
  outputBuffer: (session.output_buffer ?? session.outputBuffer ?? []).map(normalizeOutput),
  history: session.history ?? [],
})

class TerminalApiError extends Error {
  readonly status: number

  constructor(status: number, statusText: string) {
    super(`Terminal API error ${status}: ${statusText}`)
    this.status = status
  }
}

async function requestJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(path, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  })

  if (!response.ok) {
    throw new TerminalApiError(response.status, response.statusText)
  }

  return response.json() as Promise<T>
}

async function requestVoid(path: string, init?: RequestInit): Promise<void> {
  const response = await fetch(path, {
    headers: { 'Content-Type': 'application/json', ...(init?.headers ?? {}) },
    ...init,
  })

  if (!response.ok) {
    throw new TerminalApiError(response.status, response.statusText)
  }
}

async function requestJsonWithCompatibility<T>(primaryPath: string, compatibilityPath: string, init?: RequestInit): Promise<T> {
  try {
    return await requestJson<T>(primaryPath, init)
  } catch (err) {
    if (err instanceof TerminalApiError && err.status === 404) {
      return requestJson<T>(compatibilityPath, init)
    }
    throw err
  }
}

async function requestVoidWithCompatibility(primaryPath: string, compatibilityPath: string, init?: RequestInit): Promise<void> {
  try {
    await requestVoid(primaryPath, init)
  } catch (err) {
    if (err instanceof TerminalApiError && err.status === 404) {
      await requestVoid(compatibilityPath, init)
      return
    }
    throw err
  }
}

export function useTerminal() {
  const [sessions, setSessions] = useState<TerminalSession[]>([])
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null)
  const [outputs, setOutputs] = useState<Map<string, TerminalOutput[]>>(new Map())
  const [isConnected, setIsConnected] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const socketRef = useRef<WebSocket | null>(null)

  const endpoint = useMemo(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    return `${protocol}//${window.location.host}/events?role=ui-terminal-panel&client_id=lux-terminal-panel`
  }, [])

  const appendOutput = useCallback((output: TerminalOutput) => {
    setOutputs(prev => {
      const next = new Map(prev)
      next.set(output.sessionId, [...(next.get(output.sessionId) ?? []), output])
      return next
    })
  }, [])

  const refreshSessions = useCallback(async () => {
    const raw = await requestJsonWithCompatibility<RawTerminalSession[]>('/api/lux/terminal/sessions', '/api/lux/terminal/list')
    const nextSessions = raw.map(normalizeSession)
    setSessions(nextSessions)
    setOutputs(prev => {
      const next = new Map(prev)
      nextSessions.forEach(session => {
        if (!next.has(session.sessionId)) {
          next.set(session.sessionId, session.outputBuffer)
        }
      })
      return next
    })
    setActiveSessionId(current => current ?? nextSessions[0]?.sessionId ?? null)
  }, [])

  const createSession = useCallback(async () => {
    const raw = await requestJsonWithCompatibility<RawTerminalSession>('/api/lux/terminal/sessions', '/api/lux/terminal/create', { method: 'POST' })
    const session = normalizeSession(raw)
    setSessions(prev => [session, ...prev.filter(item => item.sessionId !== session.sessionId)])
    setOutputs(prev => {
      const next = new Map(prev)
      next.set(session.sessionId, session.outputBuffer)
      return next
    })
    setActiveSessionId(session.sessionId)
    return session
  }, [])

  const destroySession = useCallback(async (sessionId: string) => {
    const encodedSessionId = encodeURIComponent(sessionId)
    await requestVoidWithCompatibility(`/api/lux/terminal/sessions/${encodedSessionId}`, `/api/lux/terminal/${encodedSessionId}`, { method: 'DELETE' })
    setSessions(prev => {
      const next = prev.filter(session => session.sessionId !== sessionId)
      setActiveSessionId(current => current === sessionId ? next[0]?.sessionId ?? null : current)
      return next
    })
    setOutputs(prev => {
      const next = new Map(prev)
      next.delete(sessionId)
      return next
    })
  }, [])

  const sendInput = useCallback((sessionId: string, data: string) => {
    const message = { type: 'terminal:input', session_id: sessionId, data }
    if (socketRef.current?.readyState === WebSocket.OPEN) {
      socketRef.current.send(JSON.stringify(message))
      return true
    }
    setError('Terminal WebSocket is not connected')
    return false
  }, [])

  const connect = useCallback(() => {
    if (socketRef.current?.readyState === WebSocket.OPEN || socketRef.current?.readyState === WebSocket.CONNECTING) {
      return
    }

    const socket = new WebSocket(endpoint)
    socketRef.current = socket

    socket.addEventListener('open', () => {
      setIsConnected(true)
      setError(null)
    })

    socket.addEventListener('message', (event) => {
      try {
        const parsed = JSON.parse(String(event.data)) as TerminalWsMessage
        const payload = parsed.payload
        const type = parsed.type ?? (typeof payload?.type === 'string' ? payload.type : undefined)
        if (type !== 'terminal:output') return

        const sessionId = parsed.session_id
          ?? parsed.sessionId
          ?? (typeof payload?.session_id === 'string' ? payload.session_id : undefined)
          ?? (typeof payload?.sessionId === 'string' ? payload.sessionId : undefined)
        const data = parsed.data ?? (typeof payload?.data === 'string' ? payload.data : undefined)
        if (!sessionId || data === undefined) return

        appendOutput({
          sessionId,
          data,
          timestamp: new Date().toISOString(),
          stream: 'stdout',
        })
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err))
      }
    })

    socket.addEventListener('close', () => {
      setIsConnected(false)
      socketRef.current = null
    })

    socket.addEventListener('error', () => {
      setError('Terminal WebSocket connection error')
    })
  }, [appendOutput, endpoint])

  const disconnect = useCallback(() => {
    socketRef.current?.close()
    socketRef.current = null
    setIsConnected(false)
  }, [])

  useEffect(() => {
    void refreshSessions().catch(err => setError(err instanceof Error ? err.message : String(err)))
    connect()
    return disconnect
  }, [connect, disconnect, refreshSessions])

  return {
    sessions,
    activeSessionId,
    activeSession: sessions.find(session => session.sessionId === activeSessionId) ?? null,
    outputs,
    isConnected,
    error,
    setActiveSessionId,
    refreshSessions,
    createSession,
    destroySession,
    sendInput,
    connect,
    disconnect,
  }
}
