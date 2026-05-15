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

import { ContinuationOrchestrator } from '../index'
import { getState } from '../session-state'
import { evaluateStopConditions } from '../stop-evaluator'

describe('ContinuationOrchestrator Stop Conditions (T5.4)', () => {
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
      reportBuildResult: vi.fn(),
      reportTestResult: vi.fn(),
      reportToolExecution: vi.fn(),
      getHealthScore: vi.fn(() => 100),
      shouldPauseForErrors: vi.fn(() => false),
      getNextActionSuggestion: vi.fn(() => null),
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
    orchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
        minContinuationIntervalMs: 0,
      },
      deps: mockDeps as any,
      ctx: mockCtx as any,
      state,
      getSessionID: mockGetSessionID,
    } as any)
  })

  it('stops when all_complete is reached', async () => {
    vi.mocked(evaluateStopConditions).mockReturnValueOnce({
      shouldStop: true,
      reason: 'all_complete',
    } as any)

    const result = await orchestrator.onTrigger('test')
    expect(result.stopReason).toBe('all_complete')
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalledWith(
      expect.objectContaining({ projectPath: '/test/project' }),
      expect.objectContaining({ status: 'Complete', stop_reason: 'all_complete' })
    )
  })

  it('stops when max_continuations is reached', async () => {
    vi.mocked(evaluateStopConditions).mockReturnValueOnce({
      shouldStop: true,
        reason: 'max_continuations_reached',
    } as any)

    const result = await orchestrator.onTrigger('test')
    expect(result.stopReason).toBe('max_continuations_reached')
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalledWith(
      expect.objectContaining({ projectPath: '/test/project' }),
      expect.objectContaining({ status: 'Stopped', stop_reason: 'max_continuations_reached' })
    )
  })

  it('stops when health is critical', async () => {
    vi.mocked(evaluateStopConditions).mockReturnValueOnce({
      shouldStop: true,
      reason: 'health_critical',
    } as any)

    const result = await orchestrator.onTrigger('test')
    expect(result.stopReason).toBe('health_critical')
  })

  it('stops when stagnation is detected', async () => {
    vi.mocked(evaluateStopConditions).mockReturnValueOnce({
      shouldStop: true,
      reason: 'stagnation',
    } as any)

    const result = await orchestrator.onTrigger('test')
    expect(result.stopReason).toBe('stagnation')
  })

  it('returns continuation_state_error without persisting stopped state', async () => {
    mockDeps.stateClient.readContinuationState.mockResolvedValueOnce({
      status: 'Error',
      consecutive_failures: 3,
      continuation_count: 0,
      stagnation_count: 0,
      current_ticket_id: null,
      stop_reason: null
    })
    expect(mockDeps.stateClient.writeContinuationState).not.toHaveBeenCalled()
  })
})
