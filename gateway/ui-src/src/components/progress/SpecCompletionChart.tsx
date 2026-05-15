import React from 'react'
import type { DomainAmbiguityDatum } from '../../hooks/useProgress'

interface SpecCompletionChartProps {
  data: DomainAmbiguityDatum[]
  overallAmbiguity: number
}

function formatPercent(value: number): string {
  return `${Math.round(value * 100)}%`
}

export const SpecCompletionChart: React.FC<SpecCompletionChartProps> = ({ data, overallAmbiguity }) => {
  return (
    <section className="panel-card" aria-label="Spec completion">
      <h3 className="panel-title">Spec Completion</h3>
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', marginBottom: '12px' }}>
        <span className="stat-label">Overall completion</span>
        <span className="badge badge-info">{formatPercent(1 - overallAmbiguity)}</span>
      </div>
      <div style={{ display: 'flex', flexDirection: 'column', gap: '10px' }}>
        {data.map((domain) => (
          <div key={domain.domain}>
            <div style={{ display: 'flex', justifyContent: 'space-between', gap: '12px', marginBottom: '4px' }}>
              <span style={{ color: 'var(--text, #f5f7fb)', textTransform: 'capitalize' }}>{domain.domain.replace(/[-_]/g, ' ')}</span>
              <span style={{ color: 'var(--muted, #9aa7bc)' }}>ambiguity {formatPercent(domain.ambiguity)}</span>
            </div>
            <div
              role="img"
              aria-label={`${domain.domain} completion ${formatPercent(domain.completion)}`}
              style={{
                height: '12px',
                borderRadius: '999px',
                overflow: 'hidden',
                background: 'rgba(148, 163, 184, 0.14)',
                border: '1px solid var(--line, rgba(148, 163, 184, 0.2))',
              }}
            >
              <div
                style={{
                  width: formatPercent(domain.completion),
                  height: '100%',
                  background: 'linear-gradient(90deg, var(--green, #34d399), var(--blue, #38bdf8))',
                }}
              />
            </div>
          </div>
        ))}
        {data.length === 0 && <div className="stat-label">No spec domains found.</div>}
      </div>
    </section>
  )
}
