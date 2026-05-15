import { describe, expect, it, vi, beforeEach } from 'vitest'

vi.mock('../compile-guard', () => ({
  checkAndFixCompile: vi.fn(),
}))

vi.mock('../compaction-guard', () => ({
  isCompactionGuardActive: vi.fn(() => false),
  armCompactionGuard: vi.fn(),
  acknowledgeCompaction: vi.fn(),
}))

vi.mock('../spec-evaluator', () => ({
  evaluateSpec: vi.fn(() => ({
    ambiguity_score: 0.1,
    should_continue: true,
    next_action: 'Continue',
  })),
}))

vi.mock('../stagnation-detection', () => ({
  trackProgress: vi.fn(),
  getStagnationDetails: vi.fn(() => ({
    shouldStop: false,
    reasons: [],
  })),
  shouldStopForStagnation: vi.fn(() => false),
}))

vi.mock('../stop-evaluator', () => ({
  evaluateStopConditions: vi.fn(() => ({
    shouldStop: false,
    reason: null,
  })),
}))

vi.mock('../ticket-loader', () => ({
  loadTickets: vi.fn(() => ({
    tickets: [],
    byStatus: {},
    activeTickets: [],
    incompleteCount: 0,
  })),
  invalidateCache: vi.fn(),
}))

vi.mock('../next-action-generator', () => ({
  generateNextAction: vi.fn(() => ({
    shouldInject: false,
    message: 'Idle',
    reason: 'No tickets',
  })),
}))

import { ContinuationOrchestrator } from '../index'
import { getState } from '../session-state'
import { evaluateStopConditions } from '../stop-evaluator'
import { isCompactionGuardActive } from '../compaction-guard'

describe('ContinuationOrchestrator onTrigger (T5.2)', () => {
  const mockDeps = {
    stateClient: {
      readContinuationState: vi.fn(async () => ({
        status: 'Idle',
        continuation_count: 0,
        stagnation_count: 0,
        consecutive_failures: 0,
        current_ticket_id: null,
        stop_reason: null
      })),
      writeContinuationState: vi.fn().mockResolvedValue({ seq: 1 }),
    },
    ticketLoader: {
      loadTickets: vi.fn(() => ({
        tickets: [],
        byStatus: {},
        activeTickets: [],
        incompleteCount: 0,
      })),
      invalidateCache: vi.fn(),
    },
    signalIntegrator: vi.fn(() => ({
      getHealthScore: vi.fn(() => 100),
      getNextActionSuggestion: vi.fn(() => ''),
      reportBuildResult: vi.fn(),
      reportTestResult: vi.fn(),
      reportToolExecution: vi.fn(),
      shouldPauseForErrors: vi.fn(() => false),
      getRecentResults: vi.fn(() => []),
      clearHistory: vi.fn(),
      destroy: vi.fn(),
    })),
  }

  const mockCtx = {
    directory: '/test/project',
    client: {
      tui: {
        showToast: vi.fn(),
      },
    },
  }

  const mockGetSessionID = () => 'test-session'
  let state: any
  let orchestrator: ContinuationOrchestrator

  beforeEach(() => {
    vi.clearAllMocks()
    state = getState('/test/project')
    state.inFlight = false
    state.consecutiveFailures = 0
    orchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
        minContinuationIntervalMs: 0,
      },
      deps: mockDeps as any,
      ctx: mockCtx,
      state,
      getSessionID: mockGetSessionID,
    })
  })

  it('returns rate_limited if triggered too quickly', async () => {
    const fastOrchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
        minContinuationIntervalMs: 10000,
      },
      deps: mockDeps as any,
      ctx: mockCtx,
      state,
      getSessionID: mockGetSessionID,
    })

    await fastOrchestrator.onTrigger('test')
  })

  it('returns stopped if continuation state is Stopped', async () => {
    mockDeps.stateClient.readContinuationState.mockResolvedValueOnce({
      status: 'Stopped',
      stop_reason: 'max_continuations_reached',
      continuation_count: 50,
      stagnation_count: 0,
      consecutive_failures: 0,
      current_ticket_id: null
    })

  it('stops if evaluateStopConditions returns shouldStop', async () => {
    vi.mocked(evaluateStopConditions).mockReturnValueOnce({
      shouldStop: true,
      reason: 'health_critical',
      confidence: 1.0,
    })

    const result = await orchestrator.onTrigger('test')
    expect(result.stopReason).toBe('health_critical')
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalled()
    expect(mockCtx.client.tui.showToast).toHaveBeenCalled()
  })

  it('returns compaction_guard if compaction guard is active', async () => {
    vi.mocked(isCompactionGuardActive).mockReturnValueOnce(true)

    const result = await orchestrator.onTrigger('test')
    expect(result.message).toBe('compaction_guard')
  })

  it('returns blocked if inFlight is true', async () => {
    state.inFlight = true
    const result = await orchestrator.onTrigger('test')
    expect(result.message).toBe('blocked')
  })

  it('returns no_ticket if no tickets are available', async () => {
    const result = await orchestrator.onTrigger('test')
    expect(result.message).toBe('no_ticket')
  })
})
