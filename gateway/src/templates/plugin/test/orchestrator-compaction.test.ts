import { describe, expect, it, vi, beforeEach } from 'vitest'

vi.mock('../compile-guard', () => ({
  checkAndFixCompile: vi.fn(),
}))

vi.mock('../compaction-guard', () => ({
  isCompactionGuardActive: vi.fn(() => false),
  armCompactionGuard: vi.fn(),
  acknowledgeCompaction: vi.fn(),
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

vi.mock('../gateway-spawn', () => ({
  ensureGatewayRunning: vi.fn().mockResolvedValue(undefined),
}))

import plugin from '../index'
import { getState } from '../session-state'
import { armCompactionGuard, acknowledgeCompaction } from '../compaction-guard'

describe('Orchestrator Compaction (T5.5)', () => {
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

  it('handles experimental.session.compacting and injects context', async () => {
    const server = await plugin.server(mockCtx as any)
    const output: any = { context: [] }
    
    await server['experimental.session.compacting']({}, output)

    expect(armCompactionGuard).toHaveBeenCalled()
    expect(acknowledgeCompaction).toHaveBeenCalled()
    expect(output.context).toHaveLength(1)
    expect(output.context[0]).toContain('Continuation:')
  })

  it('handles session.compacted event', async () => {
    const server = await plugin.server(mockCtx as any)
    const state = getState('/test/project')
    
    await server.event({
      event: {
        type: 'session.compacted',
        properties: {},
      },
    })

    expect(armCompactionGuard).toHaveBeenCalledWith(state)
  })
})
