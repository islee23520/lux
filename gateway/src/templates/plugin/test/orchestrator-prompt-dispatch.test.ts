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
    tickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
    byStatus: { ToDo: 1 },
    activeTickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
    incompleteCount: 1,
  })),
  invalidateCache: vi.fn(),
}))

vi.mock('../next-action-generator', () => ({
  generateNextAction: vi.fn(() => ({
    shouldInject: true,
    message: 'Next Action',
    reason: 'Work needed',
  })),
}))

vi.mock('../prompt-builder', () => ({
  buildContinuationPrompt: vi.fn(() => 'Mocked Prompt'),
}))

import { ContinuationOrchestrator } from '../index'
import { getState } from '../session-state'

describe('ContinuationOrchestrator Prompt Dispatch (T5.3)', () => {
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
        tickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
        byStatus: { ToDo: 1 },
        activeTickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
        incompleteCount: 1,
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

  const mockPromptAsync = vi.fn().mockResolvedValue({})
  const mockCtx = {
    directory: '/test/project',
    client: {
      session: {
        promptAsync: mockPromptAsync,
      },
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
      ctx: mockCtx as any,
      state,
      getSessionID: mockGetSessionID,
    })
  })

  it('dispatches prompt successfully and updates state', async () => {
    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(true)
    expect(result.selectedTicketId).toBe('T1')
    expect(mockPromptAsync).toHaveBeenCalledWith(expect.objectContaining({
      body: { parts: [{ type: 'text', text: 'Mocked Prompt' }] },
    }))
    expect(state.continuationCount).toBe(1)
    expect(state.inFlight).toBe(false)
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalled()
    expect(mockDeps.ticketLoader.invalidateCache).toHaveBeenCalled()
  })

  it('handles promptAsync failure and increments consecutiveFailures', async () => {
    mockPromptAsync.mockRejectedValueOnce(new Error('Network error'))

    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(false)
    expect(state.consecutiveFailures).toBe(1)
    expect(state.inFlight).toBe(false)
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalled()
  })

  it('persists stopped state when prompt dispatch aborts on auth errors', async () => {
    mockPromptAsync.mockRejectedValueOnce(Object.assign(new Error('Unauthorized'), { status: 401 }))

    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(false)
    expect(mockDeps.stateClient.writeContinuationState).toHaveBeenCalledWith(
      expect.objectContaining({ projectPath: '/test/project' }),
      expect.objectContaining({ status: 'Stopped', stop_reason: 'auth' }),
    )
  })

  it('returns promptAsync unavailable if client is missing', async () => {
    const orchestratorNoClient = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
        minContinuationIntervalMs: 0,
      },
      deps: mockDeps as any,
      ctx: { directory: '/test/project' } as any,
      state,
      getSessionID: mockGetSessionID,
    })

    const result = await orchestratorNoClient.onTrigger('test')
    expect(result.message).toBe('promptAsync unavailable')
    expect(state.consecutiveFailures).toBe(1)
  })

  it('selects highest priority in-progress ticket ahead of todo tickets', async () => {
    mockDeps.ticketLoader.loadTickets.mockReturnValueOnce({
      tickets: [
        { id: 'T-low', status: 'InProgress', priority: 'Low' },
        { id: 'T-high', status: 'InProgress', priority: 'High' },
        { id: 'T-critical-todo', status: 'ToDo', priority: 'Critical' },
      ],
      byStatus: { ToDo: 1, InProgress: 2 },
      activeTickets: [
        { id: 'T-low', status: 'InProgress', priority: 'Low' },
        { id: 'T-high', status: 'InProgress', priority: 'High' },
        { id: 'T-critical-todo', status: 'ToDo', priority: 'Critical' },
      ],
      incompleteCount: 3,
    } as any)

    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(true)
    expect(result.selectedTicketId).toBe('T-high')
  })

  it('uses blocker count to break equal-priority todo ties', async () => {
    mockDeps.ticketLoader.loadTickets.mockReturnValueOnce({
      tickets: [
        { id: 'T-blocked-by-two', status: 'ToDo', priority: 'Medium', blockers: ['A', 'B'] },
        { id: 'T-unblocked', status: 'Todo', priority: 'Medium', blockers: 'not-an-array' },
      ],
      byStatus: { ToDo: 2 },
      activeTickets: [
        { id: 'T-blocked-by-two', status: 'ToDo', priority: 'Medium', blockers: ['A', 'B'] },
        { id: 'T-unblocked', status: 'Todo', priority: 'Medium', blockers: 'not-an-array' },
      ],
      incompleteCount: 2,
    } as any)

    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(true)
    expect(result.selectedTicketId).toBe('T-unblocked')
  })

  it('treats unknown or non-string priority as lowest priority', async () => {
    mockDeps.ticketLoader.loadTickets.mockReturnValueOnce({
      tickets: [
        { id: 'T-numeric-priority', status: 'ToDo', priority: 1 },
        { id: 'T-unknown-priority', status: 'ToDo', priority: 'Soon' },
        { id: 'T-low', status: 'ToDo', priority: 'Low' },
      ],
      byStatus: { ToDo: 3 },
      activeTickets: [
        { id: 'T-numeric-priority', status: 'ToDo', priority: 1 },
        { id: 'T-unknown-priority', status: 'ToDo', priority: 'Soon' },
        { id: 'T-low', status: 'ToDo', priority: 'Low' },
      ],
      incompleteCount: 3,
    } as any)

    const result = await orchestrator.onTrigger('test')

    expect(result.dispatched).toBe(true)
    expect(result.selectedTicketId).toBe('T-low')
  })

  it('returns no_ticket when active tickets are neither in-progress nor todo', async () => {
    mockDeps.ticketLoader.loadTickets.mockReturnValueOnce({
      tickets: [{ id: 'T-review', status: 'Review', priority: 'Critical' }],
      byStatus: { Review: 1 },
      activeTickets: [{ id: 'T-review', status: 'Review', priority: 'Critical' }],
      incompleteCount: 1,
    } as any)

    const result = await orchestrator.onTrigger('test')

    expect(result).toMatchObject({
      dispatched: false,
      selectedTicketId: null,
      message: 'no_ticket',
    })
    expect(mockPromptAsync).not.toHaveBeenCalled()
  })
})
