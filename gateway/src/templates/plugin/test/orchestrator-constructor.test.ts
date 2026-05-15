import { describe, expect, it, vi } from 'vitest'

vi.mock('../compile-guard', () => ({
  checkAndFixCompile: vi.fn(),
}))

import { ContinuationOrchestrator } from '../index'
import { getState } from '../session-state'

vi.mock('../session-state', () => ({
  getState: vi.fn((path) => ({
    projectPath: path,
    continuationCount: 0,
    stagnationCount: 0,
    consecutiveFailures: 0,
    inFlight: false,
  })),
}))

describe('ContinuationOrchestrator Constructor (T5.1)', () => {
  const mockDeps = {
    stateClient: {
        readContinuationState: vi.fn().mockResolvedValue({ status: 'Idle' }),
      writeContinuationState: vi.fn(),
    },
    ticketLoader: {
      loadTickets: vi.fn(),
      invalidateCache: vi.fn(),
    },
    signalIntegrator: vi.fn(),
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

  it('initializes with default config values', () => {
    const state = getState('/test/project')
    const orchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
      },
      deps: mockDeps,
      ctx: mockCtx,
      state,
      getSessionID: mockGetSessionID,
    })

    expect(orchestrator).toBeDefined()
    expect(orchestrator.isProcessing()).toBe(false)
  })

  it('overrides default config with custom values', () => {
    const state = getState('/test/project')
    const orchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:9999',
        maxContinuations: 100,
        healthThreshold: 50,
      },
      deps: mockDeps,
      ctx: mockCtx,
      state,
      getSessionID: mockGetSessionID,
    })

    expect(orchestrator).toBeDefined()
  })

  it('correctly links dependencies and state', () => {
    const state = getState('/test/project')
    state.inFlight = true
    
    const orchestrator = new ContinuationOrchestrator({
      config: {
        projectPath: '/test/project',
        gatewayUrl: 'http://localhost:18766',
      },
      deps: mockDeps,
      ctx: mockCtx,
      state,
      getSessionID: mockGetSessionID,
    })

    expect(orchestrator.isProcessing()).toBe(true)
  })
})
