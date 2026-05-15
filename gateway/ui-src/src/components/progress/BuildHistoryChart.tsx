import React from 'react'
import type { BuildHistoryDatum } from '../../hooks/useProgress'

interface BuildHistoryChartProps {
  data: BuildHistoryDatum[]
}

const statusColor: Record<BuildHistoryDatum['status'], string> = {
  Queued: '#94a3b8',
  Running: '#38bdf8',
  Succeeded: '#34d399',
  Failed: '#fb7185',
  Cancelled: '#facc15',
}

function formatTime(value: string | null): string {
  if (!value) return 'pending'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return value
  return date.toLocaleString()
}

export const BuildHistoryChart: React.FC<BuildHistoryChartProps> = ({ data }) => {
  const width = Math.max(260, data.length * 42)
  const points = data.map((build, index) => {
    const x = 24 + index * ((width - 48) / Math.max(1, data.length - 1))
    const y = build.status === 'Succeeded' ? 40 : build.status === 'Failed' ? 92 : 66
    return { build, x, y }
  })

  return (
    <section className="panel-card" aria-label="Build history">
      <h3 className="panel-title">Build History</h3>
      <div style={{ overflowX: 'auto' }}>
        <svg width={width} height="132" viewBox={`0 0 ${width} 132`} role="img" aria-label="Build success and failure timeline">
          <line x1="16" y1="66" x2={width - 16} y2="66" stroke="var(--line, rgba(148, 163, 184, 0.2))" />
          <text x="16" y="34" fill="var(--green, #34d399)" fontSize="10">success</text>
          <text x="16" y="107" fill="var(--red, #fb7185)" fontSize="10">fail</text>
          {points.map(({ build, x, y }) => (
            <g key={build.id}>
              <line x1={x} y1="40" x2={x} y2="92" stroke="rgba(148, 163, 184, 0.12)" />
              <circle cx={x} cy={y} r={build.status === 'Running' ? 8 : 6} fill={statusColor[build.status]}>
                <title>{`${build.label}: ${build.status} (${formatTime(build.completedAt ?? build.startedAt)})`}</title>
              </circle>
              <text x={x} y="122" textAnchor="middle" fill="var(--muted, #9aa7bc)" fontSize="9">
                {build.label}
              </text>
            </g>
          ))}
        </svg>
      </div>
      {data.length === 0 && <div className="stat-label">No build jobs found.</div>}
    </section>
  )
}
