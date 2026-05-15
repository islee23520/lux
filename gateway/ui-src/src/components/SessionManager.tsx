import { useState, useEffect, useCallback } from 'react'
import type { RemoteSession, ToolSession } from '../types'

function getGatewayToken(): string {
  return localStorage.getItem('lux-gateway-token') || ''
}

function authHeaders(): HeadersInit {
  const token = getGatewayToken()
  return token ? { 'x-lux-token': token } : {}
}

interface SessionManagerProps {
  onSessionSelect?: (sessionId: string) => void
}

export function SessionManager({ onSessionSelect }: SessionManagerProps) {
  const [remoteSessions, setRemoteSessions] = useState<RemoteSession[]>([])
  const [toolSessions, setToolSessions] = useState<ToolSession[]>([])
  const [remoteWebrtcEnabled, setRemoteWebrtcEnabled] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [activeTab, setActiveTab] = useState<'remote' | 'tools'>('tools')

  const fetchSessions = useCallback(async () => {
    try {
      setLoading(true)
      setError(null)

      const headers = authHeaders()

      const [flagsRes, toolRes] = await Promise.allSettled([
        fetch('/api/lux/experimental-flags'),
        fetch('/api/tools/sessions', { headers }),
      ])

      const remoteEnabled = flagsRes.status === 'fulfilled' && flagsRes.value.ok
        ? Boolean((await flagsRes.value.json()).remoteWebrtc)
        : false
      setRemoteWebrtcEnabled(remoteEnabled)

      if (remoteEnabled) {
        const remoteRes = await fetch('/api/remote/sessions', { headers })
        if (remoteRes.ok) {
          const data = await remoteRes.json()
          setRemoteSessions(Array.isArray(data) ? data : [])
        } else if (remoteRes.status === 401) {
          setError('Authentication required — set gateway token in browser storage')
        } else {
          setRemoteSessions([])
        }
      } else {
        // remote/WebRTC is hidden experimental by default; avoid calling disabled APIs or advertising it in normal navigation.
        setRemoteSessions([])
        if (activeTab === 'remote') setActiveTab('tools')
      }

      if (toolRes.status === 'fulfilled' && toolRes.value.ok) {
        const data = await toolRes.value.json()
        setToolSessions(Array.isArray(data) ? data : [])
      } else {
        setToolSessions([])
      }
    } catch {
      setError('Unable to reach gateway')
    } finally {
      setLoading(false)
    }
  }, [])

  useEffect(() => {
    fetchSessions()
  }, [fetchSessions])

  const handleCreateSession = async () => {
    if (!remoteWebrtcEnabled) {
      setError('Remote/WebRTC is hidden experimental and disabled by default')
      return
    }
    try {
      setLoading(true)
      setError(null)
      const response = await fetch('/api/remote/sessions', {
        method: 'POST',
        headers: authHeaders(),
      })
      if (!response.ok) {
        throw new Error(response.status === 401
          ? 'Authentication required'
          : 'Failed to create session')
      }
      const newSession: RemoteSession = await response.json()
      setRemoteSessions([...remoteSessions, newSession])
      if (onSessionSelect) onSessionSelect(newSession.id)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error')
    } finally {
      setLoading(false)
    }
  }

  return (
    <section aria-label="Session Manager" className="lux-panel h-full flex flex-col">
      <header className="lux-panel-header flex-col items-stretch gap-4 sm:flex-row sm:items-center">
        <h2 className="font-stencil text-[var(--text-title)] m-0">Sessions</h2>
        
        <div className="flex flex-col sm:flex-row gap-4 items-center justify-between w-full sm:w-auto">
          <div className="flex bg-[var(--color-surface)] p-1 rounded-sm border border-[var(--color-line)]" role="tablist">
            <button
              type="button"
              role="tab"
              aria-selected={activeTab === 'tools'}
              aria-controls="tools-panel"
              className={`px-4 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm transition-colors ${activeTab === 'tools' ? 'bg-[var(--color-surface-raised)] text-[var(--color-text)] shadow-sm' : 'text-[var(--color-text-muted)] hover:text-[var(--color-text)]'}`}
              onClick={() => setActiveTab('tools')}
            >
              Tool Sessions ({toolSessions.length})
            </button>
            {remoteWebrtcEnabled && (
              <button
                type="button"
                role="tab"
                aria-selected={activeTab === 'remote'}
                aria-controls="remote-panel"
                className={`px-4 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm transition-colors ${activeTab === 'remote' ? 'bg-[var(--color-surface-raised)] text-[var(--color-text)] shadow-sm' : 'text-[var(--color-text-muted)] hover:text-[var(--color-text)]'}`}
                onClick={() => setActiveTab('remote')}
              >
                Remote ({remoteSessions.length})
              </button>
            )}
          </div>
          
          <div className="flex gap-2">
            {remoteWebrtcEnabled && activeTab === 'remote' && (
              <button 
                type="button"
                onClick={handleCreateSession} 
                disabled={loading} 
                className="px-3 py-1.5 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                Create Remote Session
              </button>
            )}
            <button 
              type="button"
              onClick={fetchSessions} 
              disabled={loading} 
              className="px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              Refresh
            </button>
          </div>
        </div>
      </header>

      <div className="lux-panel-body flex-1 overflow-y-auto">
        {error && (
          <div className="mb-4 p-3 bg-red-500/10 border border-red-500/30 text-red-400 font-terminal text-[var(--text-caption)] rounded-sm">
            {error}
          </div>
        )}

        {activeTab === 'tools' && (
          <div id="tools-panel" role="tabpanel" aria-labelledby="tools-tab" className="flex flex-col gap-3">
            {toolSessions.length === 0 && !loading && (
              <p className="text-[var(--color-text-muted)] text-center p-8 font-terminal text-[var(--text-body)] italic">
                No tool sessions. Start an AI tool to create one.
              </p>
            )}
            {toolSessions.map(session => (
              <div key={session.id} className="p-4 border border-[var(--color-line)] rounded-sm bg-[var(--color-surface-raised)] hover:border-[var(--color-line-strong)] transition-colors">
                <div className="flex justify-between items-center mb-2">
                  <strong className="font-terminal text-[var(--text-body)] text-[var(--color-text)]">{session.toolType}</strong>
                  <span className={`sys-tag ${
                    session.status === 'connected' ? 'border-green-500/30 text-green-400 bg-green-500/10' : 
                    session.status === 'error' ? 'border-red-500/30 text-red-400 bg-red-500/10' : 
                    'border-[var(--color-line)] text-[var(--color-text-muted)]'
                  }`}>
                    {session.status}
                  </span>
                </div>
                {session.commandHistory.length > 0 && (
                  <div className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                    <span>Commands: {session.commandHistory.length}</span>
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {remoteWebrtcEnabled && activeTab === 'remote' && (
          <div id="remote-panel" role="tabpanel" aria-labelledby="remote-tab" className="flex flex-col gap-3">
            {remoteSessions.length === 0 && !loading && (
              <p className="text-[var(--color-text-muted)] text-center p-8 font-terminal text-[var(--text-body)] italic">
                No active remote sessions.
              </p>
            )}
            {remoteSessions.map(session => (
              <div key={session.id} className="p-4 border border-[var(--color-line)] rounded-sm bg-[var(--color-surface-raised)] hover:border-[var(--color-line-strong)] transition-colors flex justify-between items-center">
                <div>
                  <div className="flex items-center gap-3 mb-2">
                    <strong className="font-terminal text-[var(--text-body)] text-[var(--color-text)]">Session ID:</strong> 
                    <span className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">{session.id.substring(0, 8)}...</span>
                    <span className={`sys-tag ${
                      session.status === 'connected' ? 'border-green-500/30 text-green-400 bg-green-500/10' : 
                      'border-[var(--color-line)] text-[var(--color-text-muted)]'
                    }`}>
                      {session.status}
                    </span>
                  </div>
                  <div className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                    <span>Created: {new Date(session.createdAtUtc).toLocaleString()}</span>
                  </div>
                </div>
                {onSessionSelect && (
                  <button
                    type="button"
                    onClick={() => onSessionSelect(session.id)}
                    className="px-3 py-1.5 bg-[var(--color-surface)] border border-[var(--color-line)] text-[var(--color-text)] font-terminal text-[var(--text-caption)] rounded-sm hover:border-[var(--color-line-strong)] transition-colors"
                  >
                    Connect
                  </button>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  )
}
