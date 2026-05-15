import { useCallback, useEffect, useState } from 'react'
import type { AiLogEntry } from '../types'
import { useAiLogApi } from '../hooks/useAiLogApi'

function formatRelativeTime(isoUtc: string): string {
  const now = Date.now()
  const then = new Date(isoUtc).getTime()
  const diff = Math.max(0, Math.floor((now - then) / 1000))
  if (diff < 60) return `${diff}s ago`
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`
  return `${Math.floor(diff / 86400)}d ago`
}

const actorColors: Record<string, string> = {
  user: '#3b82f6',
  ai: '#8b5cf6',
  system: '#6b7280',
  remote: '#f97316',
}

const severityIcons: Record<string, string> = {
  info: '\u2139\ufe0e',
  warning: '\u26a0\ufe0f',
  error: '\u274c',
  debug: '\ud83d\udcdd',
}

const ACTOR_OPTIONS = ['All', 'user', 'ai', 'system', 'remote'] as const
const LIMIT_OPTIONS = [20, 50, 100] as const

export function AITimeline() {
  const { fetchRecent } = useAiLogApi()
  const [entries, setEntries] = useState<AiLogEntry[]>([])
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [actorFilter, setActorFilter] = useState<string>('All')
  const [limit, setLimit] = useState<number>(50)

  const loadEntries = useCallback(async () => {
    setLoading(true)
    setError(null)
    try {
      const filters = actorFilter !== 'All' ? { actor: actorFilter } : undefined
      const result = await fetchRecent(limit, filters)
      setEntries(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load entries')
    } finally {
      setLoading(false)
    }
  }, [fetchRecent, limit, actorFilter])

  useEffect(() => {
    void loadEntries()
  }, [loadEntries])

  return (
    <div className="ai-timeline">
      <div className="ai-timeline__toolbar">
        <div className="ai-timeline__filters">
          <label>
            Actor:{' '}
            <select
              value={actorFilter}
              onChange={(e) => setActorFilter(e.target.value)}
            >
              {ACTOR_OPTIONS.map((a) => (
                <option key={a} value={a}>{a}</option>
              ))}
            </select>
          </label>
          <label>
            Limit:{' '}
            <select
              value={limit}
              onChange={(e) => setLimit(Number(e.target.value))}
            >
              {LIMIT_OPTIONS.map((n) => (
                <option key={n} value={n}>{n}</option>
              ))}
            </select>
          </label>
        </div>
        <button
          className="ai-timeline__refresh"
          onClick={() => void loadEntries()}
          disabled={loading}
        >
          {loading ? 'Loading...' : 'Refresh'}
        </button>
      </div>

      {error && <div className="ai-timeline__error">{error}</div>}

      <div className="ai-timeline__list">
        {entries.length === 0 && !loading && (
          <div className="ai-timeline__empty">No entries</div>
        )}
        {entries.map((entry) => (
          <div key={entry.id} className="ai-timeline__entry">
            <span className="ai-timeline__time">
              {formatRelativeTime(entry.timestamp_utc)}
            </span>
            <span
              className="ai-timeline__actor"
              style={{ backgroundColor: actorColors[entry.actor] ?? '#6b7280' }}
            >
              {entry.actor}
            </span>
            <span className="ai-timeline__severity">
              {severityIcons[entry.severity] ?? severityIcons.info}
            </span>
            <span className="ai-timeline__action">{entry.action}</span>
            <span className="ai-timeline__target" title={entry.target}>
              {entry.target.length > 40 ? entry.target.slice(0, 40) + '\u2026' : entry.target}
            </span>
            <span className="ai-timeline__message" title={entry.message}>
              {entry.message.length > 60 ? entry.message.slice(0, 60) + '\u2026' : entry.message}
            </span>
          </div>
        ))}
      </div>
    </div>
  )
}
