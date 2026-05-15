import { useMemo } from 'react'
import type { ProgressSummaryData } from '../../hooks/useSpecKanbanProgress'

type DomainData = ProgressSummaryData['spec']['domains'][string]

interface SpecProgressBarProps {
  domains: Record<string, DomainData>
  overallAmbiguity?: number
}

export function SpecProgressBar({ domains, overallAmbiguity }: SpecProgressBarProps) {
  const domainList = useMemo(() => {
    return Object.entries(domains).map(([name, data]) => ({
      name,
      ...data,
    }))
  }, [domains])

  if (domainList.length === 0) {
    return (
      <div className="lux-spec-progress lux-spec-progress--empty">
        <span className="lux-spec-progress__empty-text">No domains</span>
      </div>
    )
  }

  return (
    <div className="lux-spec-progress" title={overallAmbiguity !== undefined ? `Overall Ambiguity: ${Math.round(overallAmbiguity * 100)}%` : undefined}>
      <div className="lux-spec-progress__bar">
        {domainList.map((domain) => {
          const statusClass = domain.status.toLowerCase()
          const label = domain.name.substring(0, 3).toUpperCase()
          const ambiguityPct = Math.round(domain.ambiguity * 100)
          
          return (
            <div
              key={domain.name}
              className={`lux-spec-progress__segment lux-spec-progress__segment--${statusClass}`}
              title={`${domain.name}\nStatus: ${domain.status}\nAmbiguity: ${ambiguityPct}%\nRequirements: ${domain.requirements_done}/${domain.requirements_total}`}
            >
              <span className="lux-spec-progress__label">{label}</span>
            </div>
          )
        })}
      </div>
    </div>
  )
}
