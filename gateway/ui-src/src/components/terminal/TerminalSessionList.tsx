import type { TerminalSession } from '../../hooks/useTerminal'

interface TerminalSessionListProps {
  sessions: TerminalSession[]
  activeSessionId: string | null
  isConnected: boolean
  onSelectSession: (sessionId: string) => void
  onCreateSession: () => void
  onDestroySession: (sessionId: string) => void
}

const formatCreatedAt = (value: string) => {
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return 'Unknown time'
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

export function TerminalSessionList({
  sessions,
  activeSessionId,
  isConnected,
  onSelectSession,
  onCreateSession,
  onDestroySession,
}: TerminalSessionListProps) {
  return (
    <aside className="terminal-session-list" aria-label="Terminal sessions">
      <div className="terminal-session-list__header">
        <div>
          <p className="terminal-session-list__eyebrow">Terminal</p>
          <h3>Sessions</h3>
        </div>
        <span className={`terminal-session-list__status ${isConnected ? 'is-connected' : 'is-closed'}`}>
          {isConnected ? 'WS' : 'Offline'}
        </span>
      </div>

      <button className="terminal-session-list__create" type="button" onClick={onCreateSession}>
        + New session
      </button>

      <div className="terminal-session-list__items">
        {sessions.length === 0 ? (
          <p className="terminal-session-list__empty">No terminal sessions yet.</p>
        ) : sessions.map(session => {
          const isActive = session.sessionId === activeSessionId
          return (
            <div
              className={`terminal-session-list__item ${isActive ? 'is-active' : ''}`}
              key={session.sessionId}
            >
              <button
                className="terminal-session-list__select"
                type="button"
                onClick={() => onSelectSession(session.sessionId)}
                aria-pressed={isActive}
              >
                <span className="terminal-session-list__name">
                  {session.sessionId.slice(0, 8)}
                </span>
                <span className="terminal-session-list__meta">
                  {session.status} · {formatCreatedAt(session.createdAt)}
                </span>
              </button>
              <button
                className="terminal-session-list__destroy"
                type="button"
                onClick={() => onDestroySession(session.sessionId)}
                aria-label={`Destroy terminal session ${session.sessionId}`}
              >
                ×
              </button>
            </div>
          )
        })}
      </div>
    </aside>
  )
}
