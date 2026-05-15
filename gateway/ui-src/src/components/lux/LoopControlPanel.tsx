import { useState } from 'react'
import { useLoopControl } from '../../hooks/useLoopControl'
import { FeedbackPanel } from '../player/FeedbackPanel'

interface LoopControlPanelProps {
  projectPath?: string | null
}

export function LoopControlPanel({ projectPath }: LoopControlPanelProps) {
  const loop = useLoopControl(projectPath)
  const [errorExpanded, setErrorExpanded] = useState(false)
  const [playStartedLoading, setPlayStartedLoading] = useState(false)

  const phase = loop.status?.current_phase ?? 'Idle'
  const isAwaitingFeedback =
    phase === 'AwaitingFeedback' ||
    phase === 'ProcessingFeedback' ||
    phase === 'Feedback'

  const handleRecordPlayStarted = async () => {
    setPlayStartedLoading(true)
    try {
      await loop.recordPlayStarted()
      await loop.refresh()
    } finally {
      setPlayStartedLoading(false)
    }
  }

  return (
    <section className="lux-panel flex flex-col gap-4" aria-label="Loop control panel">
      <header className="lux-panel-header">
        <div>
          <h3 className="font-stencil text-[var(--text-title)] m-0">Loop Control</h3>
          <p className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] m-0">
            Autonomous iteration orchestration
          </p>
        </div>
      </header>

      {loop.loading && !loop.status && (
        <div className="font-terminal text-[var(--color-text-muted)]">Loading loop status...</div>
      )}

      {loop.status && (
        <>
          <div className="flex items-center gap-3 px-4 py-3 bg-[var(--color-surface-raised)] border border-[var(--color-line)] rounded-sm">
            <span className={`lux-status-bar__badge px-3 py-1 rounded-full text-sm font-bold ${phaseTailwindClass(phase)}`}>
              {phase}
            </span>
            {loop.status.iteration > 0 && (
              <span className="font-terminal text-[var(--color-text-muted)]">
                Iteration{' '}
                <strong className="text-[var(--text-title)]">
                  {loop.status.iteration}/10
                </strong>
              </span>
            )}
          </div>

          {phase === 'Building' && (
            <button
              type="button"
              className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 disabled:opacity-50 disabled:cursor-not-allowed"
              disabled={playStartedLoading}
              onClick={() => void handleRecordPlayStarted()}
            >
              {playStartedLoading ? 'Recording...' : 'Play Started'}
            </button>
          )}

          {isAwaitingFeedback && (
            <FeedbackPanel
              onSubmit={async (input) => {
                await loop.submitFeedback(input.rating ?? 0, input.text, input.issues)
                await loop.refresh()
              }}
            />
          )}

          {(loop.error || loop.status?.last_error) && (
            <div className="border border-red-500/30 bg-red-500/5 rounded-sm">
              <button
                type="button"
                className="w-full flex items-center justify-between px-4 py-2 font-terminal text-[var(--text-caption)] text-red-400"
                onClick={() => setErrorExpanded((prev) => !prev)}
              >
                <span>Loop Error</span>
                <span>{errorExpanded ? '▼' : '▶'}</span>
              </button>
              {errorExpanded && (
                <div className="px-4 pb-3 font-sans text-sm text-red-300 whitespace-pre-wrap">
                  {loop.error || loop.status?.last_error}
                  <button
                    type="button"
                    className="mt-2 ml-2 px-3 py-1 border border-red-500/40 text-red-400 font-terminal text-xs rounded-sm hover:bg-red-500/10"
                    onClick={() => void loop.refresh()}
                  >
                    Retry
                  </button>
                </div>
              )}
            </div>
          )}
        </>
      )}
    </section>
  )
}

function phaseTailwindClass(phase: string): string {
  switch (phase) {
    case 'Idle':
    case 'Playing':
      return 'bg-green-500/20 text-green-400 border border-green-500/30'
    case 'Analyzing':
    case 'Building':
      return 'bg-yellow-500/20 text-yellow-400 border border-yellow-500/30'
    case 'Error':
      return 'bg-red-500/20 text-red-400 border border-red-500/30'
    case 'AwaitingFeedback':
    case 'ProcessingFeedback':
    case 'Feedback':
      return 'bg-blue-500/20 text-blue-400 border border-blue-500/30'
    default:
      return 'bg-gray-500/20 text-gray-400 border border-gray-500/30'
  }
}
