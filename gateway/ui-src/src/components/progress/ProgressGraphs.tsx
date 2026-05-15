import React from 'react'
import { useProgress } from '../../hooks/useProgress'
import { BuildHistoryChart } from './BuildHistoryChart'
import { KanbanStatusChart } from './KanbanStatusChart'
import { PlaySessionStats } from './PlaySessionStats'
import { SpecCompletionChart } from './SpecCompletionChart'

interface ProgressGraphsProps {
  projectPath?: string | null
}

export const ProgressGraphs: React.FC<ProgressGraphsProps> = ({ projectPath }) => {
  const { data, loading, error, wsConnected, refresh } = useProgress(projectPath)

  return (
    <section className="panel-container" aria-label="Progress graphs">
      <section className="panel-card" aria-label="Progress graph controls">
        <header style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: '12px' }}>
          <div>
            <h2 className="panel-title">Progress Graphs</h2>
            <p style={{ margin: 0, color: 'var(--muted, #9aa7bc)' }}>
              Spec ambiguity, tickets, builds, and play telemetry from Lux APIs.
            </p>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: '8px' }}>
            <span className={wsConnected ? 'badge badge-success' : 'badge badge-warning'}>
              WS {wsConnected ? 'live' : 'offline'}
            </span>
            <button className="btn" type="button" onClick={() => void refresh()} disabled={loading}>
              {loading ? 'Refreshing…' : 'Refresh'}
            </button>
          </div>
        </header>
        {error && <div className="badge badge-error" style={{ marginTop: '12px' }}>{error}</div>}
      </section>

      {data && (
        <>
          <div className="grid-2">
            <SpecCompletionChart data={data.specCompletion} overallAmbiguity={data.overallAmbiguity} />
            <KanbanStatusChart data={data.kanbanStatus} />
          </div>
          <BuildHistoryChart data={data.buildHistory} />
          <PlaySessionStats data={data.playSessionStats} />
        </>
      )}
    </section>
  )
}
