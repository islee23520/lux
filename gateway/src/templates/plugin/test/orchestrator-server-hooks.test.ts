import { describe, expect, it, vi, beforeEach } from 'vitest'

vi.mock('../compile-guard', () => ({
  checkAndFixCompile: vi.fn().mockResolvedValue({ hasErrors: false }),
}))

vi.mock('../compaction-guard', () => ({
  isCompactionGuardActive: vi.fn(() => false),
  armCompactionGuard: vi.fn(),
  acknowledgeCompaction: vi.fn(),
}))

vi.mock('../continuation-state-client', () => ({
      readContinuationState: vi.fn(async () => ({
        status: 'Idle',
        continuation_count: 0,
        stagnation_count: 0,
        consecutive_failures: 0,
        current_ticket_id: null,
        stop_reason: null
      })),
  writeContinuationState: vi.fn().mockResolvedValue({ seq: 1 }),
}))

vi.mock('../gateway-spawn', () => ({
  ensureGatewayRunning: vi.fn().mockResolvedValue(undefined),
}))

let registeredSessionCleanup: (() => void) | null = null
let registeredSessionEndCallback: (() => Promise<void>) | null = null
let registeredGetProcessing: (() => boolean) | null = null

vi.mock('../session-end-detector', () => ({
  setupSessionEndDetection: vi.fn((options, onSessionEnd) => {
    registeredSessionCleanup = options.registerEvent(vi.fn()) ?? null
    registeredSessionEndCallback = onSessionEnd
    registeredGetProcessing = options.getProcessing
    return {
      destroy: vi.fn(),
      triggerManualResume: vi.fn(),
      consumePendingResume: vi.fn(),
      setProcessing: vi.fn(),
      setProjectPath: vi.fn(),
    }
  }),
}))

const progressPoller = {
  start: vi.fn(),
  stop: vi.fn(),
  poll: vi.fn().mockResolvedValue(undefined),
}

let progressPollerConfig: any

vi.mock('../progress-poller', () => ({
  createProgressPoller: vi.fn((config) => {
    progressPollerConfig = config
    return progressPoller
  }),
}))

vi.mock('../ticket-loader', () => ({
  loadTickets: vi.fn(() => ({
    tickets: [],
    byStatus: { ToDo: 0, InProgress: 0, Done: 0, Blocked: 0 },
    activeTickets: [],
    incompleteCount: 0,
    totalTickets: 0,
  })),
  invalidateCache: vi.fn(),
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

vi.mock('../next-action-generator', () => ({
  generateNextAction: vi.fn(() => ({
    shouldInject: false,
    message: 'Idle',
    reason: 'No tickets',
  })),
}))

import plugin from '../index'
import { getState, resetState } from '../session-state'
import { checkAndFixCompile } from '../compile-guard'
import { loadTickets } from '../ticket-loader'
import { createProgressPoller } from '../progress-poller'
import { evaluateStopConditions } from '../stop-evaluator'
import { generateNextAction } from '../next-action-generator'

describe('Orchestrator Server Hooks (T5.7)', () => {
  const mockCtx = {
    directory: '/test/project',
    client: {
      tui: {
        showToast: vi.fn(),
      },
    },
  }

  beforeEach(() => {
    vi.clearAllMocks()
    resetState('/test/project')
    progressPoller.poll.mockResolvedValue(undefined)
    progressPollerConfig = undefined
    registeredSessionCleanup = null
    registeredSessionEndCallback = null
    registeredGetProcessing = null
    vi.mocked(checkAndFixCompile).mockResolvedValue({ hasErrors: false } as any)
    vi.mocked(loadTickets).mockReturnValue({
      tickets: [],
      byStatus: { ToDo: 0, InProgress: 0, Done: 0, Blocked: 0 },
      activeTickets: [],
      incompleteCount: 0,
      totalTickets: 0,
    } as any)
    vi.mocked(evaluateStopConditions).mockReturnValue({
      shouldStop: false,
      reason: null,
    } as any)
  })

  it('shows initial loaded toast and starts the progress poller', async () => {
    await plugin.server(mockCtx as any)

    expect(mockCtx.client.tui.showToast).toHaveBeenCalledWith({
      body: expect.objectContaining({
        message: expect.stringContaining('Lux Autonomous Driving loaded'),
        variant: 'success',
      }),
    })
    expect(createProgressPoller).toHaveBeenCalledWith(expect.objectContaining({
      gatewayUrl: 'http://localhost:18766',
      projectPath: '/test/project',
    }))
    expect(progressPoller.start).toHaveBeenCalledTimes(1)
  })

  it('exposes processing state through the session-end detector options', async () => {
    await plugin.server(mockCtx as any)

    expect(registeredGetProcessing?.()).toBe(false)
  })

  it('handles session.idle event and triggers orchestrator', async () => {
    const server = await plugin.server(mockCtx as any)
    
    await server.event({
      event: {
        type: 'session.idle',
        properties: {},
      },
    })

    expect(loadTickets).toHaveBeenCalled()
  })

  it('handles session.status end event and stops poller', async () => {
    const server = await plugin.server(mockCtx as any)
    
    await server.event({
      event: {
        type: 'session.status',
        properties: { status: 'end' },
      },
    })

    expect(progressPoller.stop).toHaveBeenCalledTimes(1)
  })

  it('handles session.status error event and increments failures', async () => {
    const server = await plugin.server(mockCtx as any)
    const state = getState('/test/project')
    const initialFailures = state.consecutiveFailures
    
    await server.event({
      event: {
        type: 'session.status',
        properties: { status: 'error' },
      },
    })

    expect(state.consecutiveFailures).toBe(initialFailures + 1)
  })

  it('handles session.status cancelled event and sets abortDetectedAt', async () => {
    const server = await plugin.server(mockCtx as any)
    const state = getState('/test/project')
    
    await server.event({
      event: {
        type: 'session.status',
        properties: { status: 'cancelled' },
      },
    })

    expect(state.abortDetectedAt).toBeGreaterThan(0)
  })

  it('removes session-end handlers after detector cleanup', async () => {
    vi.mocked(loadTickets).mockReturnValue({
      tickets: [{ id: 'T1', title: 'Continue work', status: 'ToDo' }],
      byStatus: { ToDo: 1 },
      activeTickets: [{ id: 'T1', title: 'Continue work', status: 'ToDo' }],
      incompleteCount: 1,
      totalTickets: 1,
    } as any)
    const server = await plugin.server(mockCtx as any)
    registeredSessionCleanup?.()
    vi.mocked(loadTickets).mockClear()

    await server.event({
      event: {
        type: 'session.status',
        properties: { status: 'end' },
      },
    })
    await new Promise((resolve) => setTimeout(resolve, 0))
    await server.event({
      event: {
        type: 'session.status',
        properties: { status: 'end' },
      },
    })

    expect(loadTickets).not.toHaveBeenCalled()
  })

  it('auto-resumes from session-end detector when incomplete tickets remain', async () => {
    vi.mocked(loadTickets).mockReturnValue({
      tickets: [{ id: 'T1', title: 'Continue work', status: 'ToDo', priority: 'High' }],
      byStatus: { ToDo: 1 },
      activeTickets: [{ id: 'T1', title: 'Continue work', status: 'ToDo', priority: 'High' }],
      incompleteCount: 1,
      totalTickets: 1,
    } as any)
    await plugin.server(mockCtx as any)

    await registeredSessionEndCallback?.()

    expect(loadTickets).toHaveBeenCalled()
  })

  it('polls immediately on session.idle events', async () => {
    const server = await plugin.server(mockCtx as any)

    await server.event({
      event: {
        type: 'session.idle',
        properties: { sessionID: 'S-idle' },
      },
    })

    expect(progressPoller.poll).toHaveBeenCalledTimes(1)
  })

  it('uses the latest session id when dispatching from server events', async () => {
    const promptAsync = vi.fn().mockResolvedValue({})
    vi.mocked(loadTickets).mockReturnValue({
      tickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
      byStatus: { ToDo: 1 },
      activeTickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
      incompleteCount: 1,
      totalTickets: 1,
    } as any)
    vi.mocked(generateNextAction).mockReturnValueOnce({
      shouldInject: true,
      message: 'Dispatch from server',
      reason: 'Work needed',
    } as any)
    const server = await plugin.server({
      ...mockCtx,
      client: {
        ...mockCtx.client,
        session: { promptAsync },
      },
    } as any)

    await server.event({
      event: {
        type: 'session.idle',
        properties: { sessionID: 'S-dispatch' },
      },
    })

    expect(promptAsync).toHaveBeenCalledWith(expect.objectContaining({
      path: { id: 'S-dispatch' },
    }))
  })

  it('logs progress poller errors through the server hook', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {})
    await plugin.server(mockCtx as any)

    progressPollerConfig.onError(new Error('poll failed'))

    expect(warn).toHaveBeenCalledWith('[Lux] Progress poller error:', 'poll failed')
  })

  it('triggers orchestrator when poller reports changed tickets', async () => {
    vi.mocked(loadTickets).mockReturnValue({
      tickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
      byStatus: { ToDo: 1 },
      activeTickets: [{ id: 'T1', status: 'ToDo', priority: 'High' }],
      incompleteCount: 1,
      totalTickets: 1,
    } as any)
    await plugin.server(mockCtx as any)

    progressPollerConfig.onProgress({
      changedTickets: [{ id: 'T1', title: 'Ticket', previousStatus: 'ToDo', newStatus: 'InProgress' }],
      progressDelta: {},
      previousSummary: null,
      currentSummary: {},
      timestamp: Date.now(),
    })

    await new Promise((resolve) => setTimeout(resolve, 0))

    expect(loadTickets).toHaveBeenCalled()
  })

  it('shows error toast when compile errors persist', async () => {
    vi.mocked(checkAndFixCompile).mockResolvedValueOnce({
      hasErrors: true,
      wasFixed: false,
      errors: ['Assets/Scripts/Broken.cs(1,1): error CS1002'],
      warnings: [],
    } as any)
    const server = await plugin.server(mockCtx as any)
    mockCtx.client.tui.showToast.mockClear()

    await server.event({
      event: {
        type: 'session.idle',
        properties: { sessionID: 'S-compile' },
      },
    })

    expect(mockCtx.client.tui.showToast).toHaveBeenCalledWith({
      body: expect.objectContaining({
        message: expect.stringContaining('Compile errors persist (1)'),
        variant: 'error',
      }),
    })
  })

  it('resets compile failure count when errors are fixed', async () => {
    const state = getState('/test/project')
    state.consecutiveCompileFailures = 2
    vi.mocked(checkAndFixCompile).mockResolvedValueOnce({
      hasErrors: false,
      wasFixed: true,
      errors: [],
      warnings: ['warning CS0168'],
    } as any)
    const server = await plugin.server(mockCtx as any)
    mockCtx.client.tui.showToast.mockClear()

    await server.event({
      event: {
        type: 'session.idle',
        properties: { sessionID: 'S-fixed' },
      },
    })

    expect(state.consecutiveCompileFailures).toBe(0)
    expect(mockCtx.client.tui.showToast).toHaveBeenCalledWith({
      body: expect.objectContaining({
        message: expect.stringContaining('All compile errors fixed'),
        variant: 'success',
      }),
    })
  })
})
