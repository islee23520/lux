import { describe, it, expect, vi, beforeEach } from 'vitest'
import { trackProgress, getStagnationDetails, shouldStopForStagnation, MAX_STAGNATION } from '../stagnation-detection'
import type { LuxSessionState } from '../session-state'
import type { ExternalSignalIntegrator } from '../external-signal-integrator'

function createMockIntegrator(overrides: Partial<ExternalSignalIntegrator> = {}): ExternalSignalIntegrator {
  return {
    reportBuildResult: vi.fn(),
    reportTestResult: vi.fn(),
    reportToolExecution: vi.fn(),
    getHealthScore: vi.fn().mockReturnValue(80),
    shouldPauseForErrors: vi.fn().mockReturnValue(false),
    getNextActionSuggestion: vi.fn().mockReturnValue('Continue'),
    getRecentResults: vi.fn().mockReturnValue([]),
    clearHistory: vi.fn(),
    destroy: vi.fn(),
    ...overrides
  }
}

describe('stagnation-detection', () => {
  let state: LuxSessionState & { zeroProgressCycles?: number }

  beforeEach(() => {
    state = {
      continuationCount: 0,
      lastInjectedAt: 0,
      stagnationCount: 0,
      lastIncompleteTicketCount: -1,
      lastAmbiguityScore: NaN,
      awaitingPostInjectionProgressCheck: false,
      inFlight: false,
      consecutiveFailures: 0,
      consecutiveCompileFailures: 0,
      recentCompactionAt: null,
      recentCompactionEpoch: 0,
      acknowledgedCompactionEpoch: 0
    }
  })

  describe('trackProgress', () => {
    it('should reset stagnation count when tickets decrease', () => {
      state.lastIncompleteTicketCount = 5
      state.stagnationCount = 2
      
      trackProgress(state, [{ id: 'T1', status: 'Done' }], 10)
      
      expect(state.stagnationCount).toBe(0)
      expect(state.lastIncompleteTicketCount).toBe(0)
    })

    it('should reset stagnation count when ambiguity decreases', () => {
      state.lastAmbiguityScore = 50
      state.stagnationCount = 2
      
      trackProgress(state, [], 40)
      
      expect(state.stagnationCount).toBe(0)
      expect(state.lastAmbiguityScore).toBe(40)
    })

    it('should increment stagnation count if no progress and awaiting check', () => {
      state.lastIncompleteTicketCount = 1
      state.lastAmbiguityScore = 10
      state.stagnationCount = 0
      state.awaitingPostInjectionProgressCheck = true
      
      trackProgress(state, [{ id: 'T1', status: 'Todo' }], 10)
      
      expect(state.stagnationCount).toBe(1)
      expect(state.awaitingPostInjectionProgressCheck).toBe(false)
    })

    it('should update consecutive failures from integrator', () => {
      const integrator = {
        getRecentResults: () => [
          { type: 'build', success: false },
          { type: 'build', success: false }
        ]
      }
      
      trackProgress(state, [], 10, integrator as any)
      expect(state.consecutiveFailures).toBe(2)
    })

    it('should preserve zero-progress cycles through tracking and details', () => {
      state.lastIncompleteTicketCount = 1
      state.lastAmbiguityScore = 10
      state.awaitingPostInjectionProgressCheck = true
      state.zeroProgressCycles = 4

      trackProgress(state, [{ id: 'T1', status: 'ToDo' }], 10)

      const details = getStagnationDetails(state)
      expect(details.zeroProgressCycles).toBe(5)
      expect(details.reasons).toContain('zero_progress_cycle')
      expect(details.shouldStop).toBe(true)
    })

    it('should use provided max stagnation override and ignore low health when integrator missing', () => {
      state.stagnationCount = 1
      state.zeroProgressCycles = 1

      const details = getStagnationDetails(state, 2)

      expect(details.shouldStop).toBe(false)
      expect(details.healthScore).toBeUndefined()
      expect(details.reasons).not.toContain('ticket_stagnation')
    })

    it('should assign consecutive build failures from recent integrator results', () => {
      const integrator = createMockIntegrator({
        getRecentResults: vi.fn().mockReturnValue([
          { type: 'build', success: false, tool: 'build', timestamp: 1 },
          { type: 'test', success: true, tool: 'test', timestamp: 2 },
          { type: 'build', success: false, tool: 'build', timestamp: 3 },
          { type: 'build', success: false, tool: 'build', timestamp: 4 }
        ])
      })

      trackProgress(state, [], 10, integrator)

      expect(state.consecutiveFailures).toBe(2)
    })
  })

  describe('getStagnationDetails', () => {
    it('should detect ticket stagnation', () => {
      state.stagnationCount = MAX_STAGNATION
      const details = getStagnationDetails(state)
      expect(details.shouldStop).toBe(true)
      expect(details.reasons).toContain('ticket_stagnation')
    })

    it('should detect health degradation', () => {
      const integrator = {
        getHealthScore: () => 20,
        getRecentResults: () => []
      }
      const details = getStagnationDetails(state, integrator as any)
      expect(details.shouldStop).toBe(true)
      expect(details.reasons).toContain('health_degraded')
    })

    it('should detect build failure streak', () => {
      const integrator = {
        getHealthScore: () => 100,
        getRecentResults: () => [
          { type: 'build', success: false },
          { type: 'build', success: false },
          { type: 'build', success: false }
        ]
      }
      const details = getStagnationDetails(state, integrator as any)
      expect(details.shouldStop).toBe(true)
      expect(details.reasons).toContain('build_failure_streak')
    })

    it('should stop counting build failures at the first non-failing build result', () => {
      const integrator = createMockIntegrator({
        getRecentResults: vi.fn().mockReturnValue([
          { type: 'build', success: false, tool: 'build', timestamp: 1 },
          { type: 'test', success: true, tool: 'test', timestamp: 2 },
          { type: 'build', success: false, tool: 'build', timestamp: 3 },
          { type: 'build', success: false, tool: 'build', timestamp: 4 }
        ])
      })

      const details = getStagnationDetails(state, integrator)

      expect(details.buildFailureStreak).toBe(2)
      expect(details.shouldStop).toBe(false)
      expect(details.reasons).not.toContain('build_failure_streak')
    })

    it('should detect zero-progress cycle stagnation', () => {
      state.zeroProgressCycles = 5

      const details = getStagnationDetails(state, createMockIntegrator())

      expect(details.shouldStop).toBe(true)
      expect(details.zeroProgressCycles).toBe(5)
      expect(details.reasons).toContain('zero_progress_cycle')
      expect(shouldStopForStagnation(state)).toBe(true)
    })

    it('should support numeric max stagnation overload', () => {
      state.stagnationCount = 4

      const details = getStagnationDetails(state, 5)

      expect(details.shouldStop).toBe(false)
      expect(details.healthScore).toBeUndefined()
      expect(details.buildFailureStreak).toBe(0)
      expect(details.reasons).not.toContain('ticket_stagnation')
    })
  })

  describe('shouldStopForStagnation', () => {
    it('should return true and log warnings when stagnated', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      state.stagnationCount = MAX_STAGNATION
      
      const result = shouldStopForStagnation(state)
      
      expect(result).toBe(true)
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('ticket_stagnation'))
      warnSpy.mockRestore()
    })

    it('should log health degradation warnings', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      const integrator = createMockIntegrator({
        getHealthScore: vi.fn().mockReturnValue(20)
      })

      const result = shouldStopForStagnation(state, integrator)

      expect(result).toBe(true)
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('health_degraded'))
      warnSpy.mockRestore()
    })

    it('should log build failure streak warnings', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      const integrator = createMockIntegrator({
        getRecentResults: vi.fn().mockReturnValue([
          { type: 'build', success: false, tool: 'build', timestamp: 1 },
          { type: 'build', success: false, tool: 'build', timestamp: 2 },
          { type: 'build', success: false, tool: 'build', timestamp: 3 }
        ])
      })

      const result = shouldStopForStagnation(state, integrator)

      expect(result).toBe(true)
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('build_failure_streak'))
      warnSpy.mockRestore()
    })

    it('should log zero-progress cycle warnings', () => {
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      state.zeroProgressCycles = 5

      const result = shouldStopForStagnation(state)

      expect(result).toBe(true)
      expect(warnSpy).toHaveBeenCalledWith(expect.stringContaining('zero_progress_cycle'))
      warnSpy.mockRestore()
    })

    it('should return false when nothing is stagnating', () => {
      const result = shouldStopForStagnation(state)
      expect(result).toBe(false)
    })

    it('should stop when health degrades via integrator', () => {
      const integrator = createMockIntegrator({ getHealthScore: vi.fn().mockReturnValue(10) })
      const result = shouldStopForStagnation(state, integrator)
      expect(result).toBe(true)
    })

    it('should stop when multiple reasons combine', () => {
      state.stagnationCount = MAX_STAGNATION
      state.zeroProgressCycles = 5
      const integrator = createMockIntegrator({ getHealthScore: vi.fn().mockReturnValue(10) })

      const result = getStagnationDetails(state, integrator)
      expect(result.reasons).toEqual(expect.arrayContaining(['ticket_stagnation', 'health_degraded', 'zero_progress_cycle']))
    })
  })
})
