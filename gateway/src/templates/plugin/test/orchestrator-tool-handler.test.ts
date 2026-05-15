import { describe, expect, it, vi, beforeEach } from 'vitest'

vi.mock('../compile-guard', () => ({
  checkAndFixCompile: vi.fn(),
}))

vi.mock('../compaction-guard', () => ({
  isCompactionGuardActive: vi.fn(() => false),
  armCompactionGuard: vi.fn(),
  acknowledgeCompaction: vi.fn(),
}))

vi.mock('../gateway-spawn', () => ({
  ensureGatewayRunning: vi.fn().mockResolvedValue(undefined),
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
import { invalidateCache } from '../ticket-loader'

describe('Orchestrator Tool Handler (T5.6)', () => {
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
  })

  it('handles tool.execute.after for mutating lux tools', async () => {
    const server = await plugin.server(mockCtx as any)
    
    await server['tool.execute.after']({ tool: 'lux_ticket_create' }, { ok: true })

    expect(invalidateCache).toHaveBeenCalled()
  })

  it('ignores non-lux tools', async () => {
    const server = await plugin.server(mockCtx as any)
    
    await server['tool.execute.after']({ tool: 'ls' }, { ok: true })

    expect(invalidateCache).not.toHaveBeenCalled()
  })

  it('ignores non-mutating lux tools', async () => {
    const server = await plugin.server(mockCtx as any)
    
    await server['tool.execute.after']({ tool: 'lux_status' }, { ok: true })

    expect(invalidateCache).not.toHaveBeenCalled()
  })
})
