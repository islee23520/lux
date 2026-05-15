import React from 'react'
import type { KanbanStatusDatum } from '../../hooks/useProgress'

interface KanbanStatusChartProps {
  data: KanbanStatusDatum[]
}

const colors: Record<string, string> = {
  Backlog: '#94a3b8',
  Blocked: '#fb7185',
  ToDo: '#38bdf8',
  InProgress: '#facc15',
  Done: '#34d399',
}

function describeArc(startRatio: number, endRatio: number): string {
  const radius = 42
  const center = 50
  const startAngle = startRatio * Math.PI * 2 - Math.PI / 2
  const endAngle = endRatio * Math.PI * 2 - Math.PI / 2
  const startX = center + radius * Math.cos(startAngle)
  const startY = center + radius * Math.sin(startAngle)
  const endX = center + radius * Math.cos(endAngle)
  const endY = center + radius * Math.sin(endAngle)
  const largeArc = endRatio - startRatio > 0.5 ? 1 : 0
  return `M ${center} ${center} L ${startX} ${startY} A ${radius} ${radius} 0 ${largeArc} 1 ${endX} ${endY} Z`
}

export const KanbanStatusChart: React.FC<KanbanStatusChartProps> = ({ data }) => {
  const total = data.reduce((sum, item) => sum + item.count, 0)
  let cursor = 0

  return (
    <section className="panel-card" aria-label="Kanban status">
      <h3 className="panel-title">Kanban Status</h3>
      <div style={{ display: 'grid', gridTemplateColumns: 'minmax(140px, 180px) 1fr', gap: '16px', alignItems: 'center' }}>
        <svg viewBox="0 0 100 100" role="img" aria-label={`Ticket status distribution, ${total} total tickets`}>
          <circle cx="50" cy="50" r="42" fill="rgba(148, 163, 184, 0.12)" />
          {total > 0 &&
            data.map((item) => {
              const start = cursor / total
              cursor += item.count
              const end = cursor / total
              return <path key={item.status} d={describeArc(start, end)} fill={colors[item.status]} opacity="0.9" />
            })}
          <circle cx="50" cy="50" r="25" fill="var(--panel, rgba(13, 18, 29, 0.98))" />
          <text x="50" y="49" textAnchor="middle" fill="var(--text, #f5f7fb)" fontSize="13" fontWeight="700">
            {total}
          </text>
          <text x="50" y="61" textAnchor="middle" fill="var(--muted, #9aa7bc)" fontSize="7">
            tickets
          </text>
        </svg>
        <div style={{ display: 'flex', flexDirection: 'column', gap: '8px' }}>
          {data.map((item) => (
            <div key={item.status} style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '10px' }}>
              <span style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
                <span style={{ width: '10px', height: '10px', borderRadius: '50%', background: colors[item.status] }} />
                <span>{item.status}</span>
              </span>
              <span className="badge badge-info">{item.count}</span>
            </div>
          ))}
        </div>
      </div>
    </section>
  )
}
