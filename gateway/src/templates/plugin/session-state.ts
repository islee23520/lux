export interface LuxSessionState {
  continuationCount: number
  lastInjectedAt: number
  awaitingPostInjectionProgressCheck: boolean
  inFlight: boolean
  stagnationCount: number
  consecutiveFailures: number
  consecutiveCompileFailures: number
  recentCompactionAt: number | null
  recentCompactionEpoch: number
  acknowledgedCompactionEpoch: number
  lastIncompleteTicketCount: number
  lastAmbiguityScore: number
  abortDetectedAt?: number
  tokenLimitDetected?: boolean
}

const states = new Map<string, LuxSessionState>()

function createState(): LuxSessionState {
  return {
    continuationCount: 0,
    lastInjectedAt: 0,
    awaitingPostInjectionProgressCheck: false,
    inFlight: false,
    stagnationCount: 0,
    consecutiveFailures: 0,
    consecutiveCompileFailures: 0,
    recentCompactionAt: null,
    recentCompactionEpoch: 0,
    acknowledgedCompactionEpoch: 0,
    lastIncompleteTicketCount: -1,
    lastAmbiguityScore: Number.POSITIVE_INFINITY,
  }
}

export function getState(projectPath: string): LuxSessionState {
  let state = states.get(projectPath)
  if (!state) {
    // Keying by projectPath keeps simultaneous projects isolated without a global shared session.
    state = createState()
    states.set(projectPath, state)
  }
  return state
}

export function resetState(projectPath: string): void {
  states.delete(projectPath)
}

export function updateState(projectPath: string, patch: Partial<LuxSessionState>): LuxSessionState {
  const state = getState(projectPath)
  for (const [key, value] of Object.entries(patch) as Array<[keyof LuxSessionState, LuxSessionState[keyof LuxSessionState]]>) {
    if (value !== undefined) {
      state[key] = value as never
    }
  }
  return state
}
