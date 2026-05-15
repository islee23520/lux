import React from 'react'
import type { PlaySessionStatsData } from '../../hooks/useProgress'

interface PlaySessionStatsProps {
  data: PlaySessionStatsData
}

function formatDuration(seconds: number): string {
  const safeSeconds = Math.max(0, Math.round(seconds))
  const minutes = Math.floor(safeSeconds / 60)
  const remaining = safeSeconds % 60
  return `${minutes}m ${remaining}s`
}

export const PlaySessionStats: React.FC<PlaySessionStatsProps> = ({ data }) => {
  const maxEventTypeCount = Math.max(1, ...data.eventTypes.map((item) => item.count))

  return (
    <section className="panel-card" aria-label="Play session stats">
      <h3 className="panel-title">Play Session Stats</h3>
      <div className="grid-3" style={{ marginBottom: '16px' }}>
        <div className="stat-card">
          <div className="stat-label">Sessions</div>
          <div className="stat-value">{data.sessionCount}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Events</div>
          <div className="stat-value">{data.totalEvents}</div>
        </div>
        <div className="stat-card">
          <div className="stat-label">Avg Duration</div>
          <div className="stat-value" style={{ fontSize: '1.4em' }}>{formatDuration(data.averageDurationSecs)}</div>
        </div>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
        {data.eventTypes.map((item) => (
          <div key={item.eventType}>
            <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: '4px' }}>
              <span>{item.eventType}</span>
              <span style={{ color: 'var(--muted, #9aa7bc)' }}>{item.count}</span>
            </div>
            <div style={{ height: '8px', borderRadius: '999px', background: 'rgba(148, 163, 184, 0.14)', overflow: 'hidden' }}>
              <div style={{ height: '100%', width: `${(item.count / maxEventTypeCount) * 100}%`, background: 'var(--blue, #38bdf8)' }} />
            </div>
          </div>
        ))}
        {data.eventTypes.length === 0 && <div className="stat-label">No play events found in recent sessions.</div>}
      </div>
      <div style={{ marginTop: '12px', color: 'var(--muted, #9aa7bc)' }}>
        Total duration: {formatDuration(data.totalDurationSecs)}
      </div>
    </section>
  )
}
