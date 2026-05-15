import { useEffect, useState } from 'react'
import { useSpecKanbanProgress } from '../../hooks/useSpecKanbanProgress'
import { useLoopControl } from '../../hooks/useLoopControl'
import { SpecProgressBar } from './SpecProgressBar'
import { KanbanMiniBar } from './KanbanMiniBar'

interface LuxStatusBarProps {
  projectPath?: string | null
}

type LoopState = 'Idle' | 'Analyzing' | 'Building' | 'Playing' | 'Feedback' | 'Improving'
type BuildStatus = 'Queued' | 'Running' | 'Succeeded' | 'Cancelled' | { Failed: string }

interface BuildJobSummary {
  status: BuildStatus
}

interface StatusState {
  buildStatus: string | null
  loopState: LoopState
  error: string | null
}

function fetchJson<T>(path: string): Promise<T> {
  return fetch(path).then((res) => {
    if (!res.ok) throw new Error(`${path} failed: ${res.status}`)
    return res.json() as Promise<T>
  })
}

function normalizeBuildStatus(status: BuildStatus | undefined): string | null {
  if (!status) return null
  return typeof status === 'string' ? status : `Failed: ${status.Failed}`
}

function buildLoopState(ambiguity: number | null, activeTickets: number | null, buildStatus: string | null, loopPhase?: string | null): LoopState {
  if (loopPhase === 'AwaitingFeedback' || loopPhase === 'ProcessingFeedback') return 'Feedback'
  if (loopPhase === 'Analyzing') return 'Analyzing'
  if (loopPhase === 'Building') return 'Building'
  if (loopPhase === 'AwaitingPlay' || loopPhase === 'Playing') return 'Playing'
  if (buildStatus === 'Queued' || buildStatus === 'Running') return 'Building'
  if (ambiguity !== null && ambiguity > 0.35) return 'Analyzing'
  if (activeTickets !== null && activeTickets > 0) return 'Improving'
  if (buildStatus === 'Succeeded') return 'Playing'
  if (buildStatus?.startsWith('Failed')) return 'Feedback'
  return 'Idle'
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

export function LuxStatusBar({ projectPath }: LuxStatusBarProps) {
  const { data: progressData, error: progressError } = useSpecKanbanProgress(projectPath)
  const loopControl = useLoopControl(projectPath)
  
  const [state, setState] = useState<StatusState>({
    buildStatus: null,
    loopState: 'Idle',
    error: null,
  })
  const [loopErrorExpanded, setLoopErrorExpanded] = useState(false)

  useEffect(() => {
    let cancelled = false

    async function loadBuildStatus(): Promise<void> {
      if (!projectPath) {
        setState({ buildStatus: null, loopState: 'Idle', error: null })
        return
      }

      try {
        const builds = await fetchJson<BuildJobSummary[]>('/api/lux/build/list')
        if (cancelled) return

        const latestBuild = builds[0]
        const buildStatus = normalizeBuildStatus(latestBuild?.status)
        
        const ambiguity = progressData?.spec.overall_ambiguity ?? null
        const activeTickets = progressData?.kanban.active_count ?? null
        const loopPhase = loopControl.status?.current_phase ?? null

        setState({
          buildStatus,
          loopState: buildLoopState(ambiguity, activeTickets, buildStatus, loopPhase),
          error: null,
        })
      } catch (caught) {
        if (cancelled) return
        setState((current) => ({
          ...current,
          loopState: 'Idle',
          error: caught instanceof Error ? caught.message : 'Failed to load build status',
        }))
      }
    }

    void loadBuildStatus()
    const interval = window.setInterval(() => void loadBuildStatus(), 15000)

    return () => {
      cancelled = true
      window.clearInterval(interval)
    }
  }, [projectPath, progressData, loopControl.status?.current_phase])

  const displayError = progressError || state.error || loopControl.error

  return (
    <section className="lux-status-bar" aria-label="Lux status">
      <div className="lux-status-bar__metric lux-status-bar__metric--spec">
        <span className="lux-status-bar__label">Spec</span>
        {progressData?.spec ? (
          <SpecProgressBar 
            domains={progressData.spec.domains} 
            overallAmbiguity={progressData.spec.overall_ambiguity} 
          />
        ) : (
          <strong>—</strong>
        )}
      </div>
      <div className="lux-status-bar__metric lux-status-bar__metric--kanban">
        <span className="lux-status-bar__label">Kanban</span>
        {progressData?.kanban ? (
          <KanbanMiniBar 
            byStatus={progressData.kanban.by_status} 
            total={progressData.kanban.total} 
          />
        ) : (
          <strong>—</strong>
        )}
      </div>
      <div className="lux-status-bar__metric">
        <span className="lux-status-bar__label">Build</span>
        <strong>{state.buildStatus ?? 'No jobs'}</strong>
      </div>
      <div className="lux-status-bar__metric lux-status-bar__metric--loop">
        <span className="lux-status-bar__pulse" aria-hidden="true" />
        <span className="lux-status-bar__label">Loop</span>
        <strong>{state.loopState}</strong>
        {loopControl.status && loopControl.status.is_running && (
          <span className={`lux-status-bar__badge px-2 py-0.5 rounded-full text-xs font-bold ${phaseTailwindClass(loopControl.status.current_phase)}`}>
            {loopControl.status.current_phase}
          </span>
        )}
        {loopControl.status && loopControl.status.iteration > 0 && (
          <span className="lux-status-bar__iteration">
            Iter {loopControl.status.iteration}/10
          </span>
        )}
      </div>
      {displayError && (
        <button
          type="button"
          className="lux-status-bar__error cursor-pointer bg-transparent border-none"
          onClick={() => setLoopErrorExpanded((prev) => !prev)}
        >
          Status partial {loopErrorExpanded ? '▼' : '▶'}
        </button>
      )}
      {loopErrorExpanded && displayError && (
        <div className="w-full mt-1 p-2 bg-[var(--color-surface-raised)] border border-[var(--color-line)] rounded-sm font-sans text-xs text-red-400 whitespace-pre-wrap">
          {displayError}
        </div>
      )}
    </section>
  )
}
