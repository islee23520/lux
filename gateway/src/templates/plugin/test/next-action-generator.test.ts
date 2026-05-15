import { describe, expect, it } from 'vitest'
import { generateNextAction, NextActionContext } from '../next-action-generator'

describe('next-action-generator', () => {
  const createDefaultContext = (overrides: Partial<NextActionContext> = {}): NextActionContext => ({
    activeTickets: [],
    inactiveTickets: [],
    ticketCounts: {},
    incompleteCount: 0,
    ambiguityScore: 0.5,
    shouldContinueSpec: false,
    nextSpecAction: '',
    continuationCount: 0,
    stagnationCount: 0,
    consecutiveFailures: 0,
    lastAmbiguity: 0.5,
    isCompactionGuardActive: false,
    maxContinuations: 10,
    ...overrides,
  })

  describe('generateNextAction', () => {
    it('returns all-done state when no tickets are left', () => {
      const ctx = createDefaultContext({
        incompleteCount: 0,
        activeTickets: [],
      })
      const result = generateNextAction(ctx)

      expect(result.shouldInject).toBe(false)
      expect(result.reason).toBe('all-tickets-complete')
      expect(result.message).toContain('All spec tickets complete!')
      expect(result.confidence).toBe(0.95)
    })

    it('returns max-continuations state when limit is reached', () => {
      const ctx = createDefaultContext({
        incompleteCount: 5,
        continuationCount: 10,
        maxContinuations: 10,
      })
      const result = generateNextAction(ctx)

      expect(result.shouldInject).toBe(false)
      expect(result.reason).toBe('max-continuations-reached')
      expect(result.message).toContain('Maximum continuations reached (10)')
      expect(result.confidence).toBe(1.0)
    })

    it('returns compaction-guard-active state when guard is on', () => {
      const ctx = createDefaultContext({
        incompleteCount: 5,
        isCompactionGuardActive: true,
      })
      const result = generateNextAction(ctx)

      expect(result.shouldInject).toBe(false)
      expect(result.reason).toBe('compaction-guard-active')
      expect(result.message).toBe('')
      expect(result.confidence).toBe(0)
    })

    it('uses suggested action when no active ticket exists', () => {
      const ctx = createDefaultContext({
        incompleteCount: 1,
        activeTickets: [],
        suggestedAction: 'Investigate logs',
      })

      const result = generateNextAction(ctx)
      expect(result.message).toContain('Suggested: Investigate logs')
    })

    it('uses spec continuation reason when no tickets or suggestion exist', () => {
      const ctx = createDefaultContext({ incompleteCount: 1 })
      const result = generateNextAction(ctx)

      expect(result.reason).toBe('incomplete-tickets')
      expect(result.message).toContain('Do not ask for permission')
    })

    it('handles active tickets with priority ordering', () => {
      const ctx = createDefaultContext({
        activeTickets: [
          { id: 'T1', title: 'Task 1', status: 'Todo', priority: 'High', spec_ref: 'S1' },
          { id: 'T2', title: 'Task 2', status: 'In Progress', priority: 'Medium' },
        ],
        incompleteCount: 2,
      })
      const result = generateNextAction(ctx)

      expect(result.shouldInject).toBe(true)
      expect(result.reason).toBe('active-tickets')
      expect(result.message).toContain('Current focus (priority order):')
      expect(result.message).toContain('1. [Todo] Task 1 (S1)')
      expect(result.message).toContain('2. [In Progress] Task 2')
      expect(result.message).toContain('Next action: Continue with "Task 1"')
    })

    it('limits active ticket display to 8 items', () => {
      const activeTickets = Array.from({ length: 10 }, (_, i) => ({
        id: `T${i}`,
        title: `Task ${i}`,
        status: 'Todo',
      }))
      const ctx = createDefaultContext({ activeTickets, incompleteCount: 10 })
      const result = generateNextAction(ctx)

      expect(result.message).toContain('8. [Todo] Task 7')
      expect(result.message).not.toContain('9. [Todo] Task 8')
    })

    it('includes spec recommendation when available', () => {
      const ctx = createDefaultContext({
        activeTickets: [{ id: 'T1', status: 'Todo' }],
        incompleteCount: 1,
        shouldContinueSpec: true,
        nextSpecAction: 'Refine architecture',
        ambiguityScore: 0.25,
      })
      const result = generateNextAction(ctx)

      expect(result.message).toContain('Spec ambiguity: 25%')
      expect(result.message).toContain('Spec recommendation: Refine architecture')
    })

    it('handles low health score with suggested action', () => {
      const ctx = createDefaultContext({
        activeTickets: [{ id: 'T1', status: 'Todo' }],
        incompleteCount: 1,
        healthScore: 30,
        suggestedAction: 'Fix compiler errors',
      })
      const result = generateNextAction(ctx)

      expect(result.reason).toBe('low-health-score')
      expect(result.message).toContain('⚠️ Build health: 30/100 — Fix compiler errors')
    })

    it('triggers stagnation recovery reason', () => {
      const ctx = createDefaultContext({
        activeTickets: [{ id: 'T1', status: 'Todo' }],
        incompleteCount: 1,
        stagnationCount: 3,
      })
      const result = generateNextAction(ctx)

      expect(result.reason).toBe('stagnation-recovery')
    })

    it('falls back to incomplete-tickets reason if no active tickets', () => {
      const ctx = createDefaultContext({
        activeTickets: [],
        incompleteCount: 5,
      })
      const result = generateNextAction(ctx)

      expect(result.reason).toBe('incomplete-tickets')
    })

    it('calculates confidence correctly (max confidence)', () => {
      const ctx = createDefaultContext({
        activeTickets: [{ id: 'T1', status: 'Todo' }],
        ambiguityScore: 0.1,
        healthScore: 90,
        stagnationCount: 0,
        continuationCount: 1,
        maxContinuations: 10,
      })
      const result = generateNextAction(ctx)

      expect(result.confidence).toBe(1.0)
    })

    it('calculates confidence correctly (low confidence)', () => {
      const ctx = createDefaultContext({
        activeTickets: [],
        ambiguityScore: 0.8,
        healthScore: 20,
        stagnationCount: 5,
        continuationCount: 9,
        maxContinuations: 10,
        incompleteCount: 1,
      })
      const result = generateNextAction(ctx)

      expect(result.confidence).toBe(0.1)
    })

    it('includes footer in the message', () => {
      const ctx = createDefaultContext({
        activeTickets: [{ id: 'T1', status: 'Todo' }],
        incompleteCount: 1,
      })
      const result = generateNextAction(ctx)

      expect(result.message).toContain('Do not ask for permission. Use .lux as source of truth.')
    })
  })
})
