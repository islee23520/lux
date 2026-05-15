import { useMemo } from 'react'

interface KanbanMiniBarProps {
  byStatus: Record<string, number>
  total: number
}

const STATUS_ORDER = ['Backlog', 'Blocked', 'ToDo', 'InProgress', 'Done']

export function KanbanMiniBar({ byStatus, total }: KanbanMiniBarProps) {
  const segments = useMemo(() => {
    if (total === 0) return []

    return STATUS_ORDER.map((status) => {
      const count = byStatus[status] || 0
      if (count === 0) return null

      const percentage = (count / total) * 100
      return {
        status,
        count,
        percentage,
        className: `lux-kanban-mini__segment--${status.toLowerCase()}`,
      }
    }).filter((s): s is NonNullable<typeof s> => s !== null)
  }, [byStatus, total])

  const handleClick = (status: string) => {
    console.log(`Navigate to kanban: ${status}`)
  }

  if (total === 0) {
    return (
      <div className="lux-kanban-mini lux-kanban-mini--empty">
        <span className="lux-kanban-mini__empty-text">No tickets</span>
      </div>
    )
  }

  return (
    <div className="lux-kanban-mini" title={`Total Tickets: ${total}`}>
      <div className="lux-kanban-mini__bar">
        {segments.map((segment) => (
          <button
            key={segment.status}
            className={`lux-kanban-mini__segment ${segment.className}`}
            style={{ width: `${segment.percentage}%` }}
            title={`${segment.status}: ${segment.count}`}
            onClick={() => handleClick(segment.status)}
            type="button"
          >
            {segment.percentage > 10 && (
              <span className="lux-kanban-mini__count">{segment.count}</span>
            )}
          </button>
        ))}
      </div>
    </div>
  )
}
