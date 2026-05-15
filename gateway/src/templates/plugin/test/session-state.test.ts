import { describe, it, expect, beforeEach } from 'vitest'
import { getState, resetState, updateState } from '../session-state'

describe('session-state', () => {
  const projectPathA = '/path/to/project-a'
  const projectPathB = '/path/to/project-b'

  beforeEach(() => {
    resetState(projectPathA)
    resetState(projectPathB)
  })

  describe('getState', () => {
    it('should return a new state with default values for a new project path', () => {
      const state = getState(projectPathA)
      
      expect(state).toBeDefined()
      expect(state.continuationCount).toBe(0)
      expect(state.stagnationCount).toBe(0)
      expect(state.consecutiveFailures).toBe(0)
      expect(state.consecutiveCompileFailures).toBe(0)
      expect(state.lastInjectedAt).toBe(0)
      expect(state.awaitingPostInjectionProgressCheck).toBe(false)
      expect(state.inFlight).toBe(false)
      expect(state.recentCompactionAt).toBeNull()
      expect(state.recentCompactionEpoch).toBe(0)
      expect(state.acknowledgedCompactionEpoch).toBe(0)
      expect(state.lastIncompleteTicketCount).toBe(-1)
      expect(state.lastAmbiguityScore).toBe(Number.POSITIVE_INFINITY)
    })

    it('should return the same state instance for the same project path', () => {
      const state1 = getState(projectPathA)
      state1.continuationCount = 5
      
      const state2 = getState(projectPathA)
      expect(state2).toBe(state1)
      expect(state2.continuationCount).toBe(5)
    })

    it('should isolate states between different project paths', () => {
      const stateA = getState(projectPathA)
      const stateB = getState(projectPathB)
      
      expect(stateA).not.toBe(stateB)
      
      stateA.continuationCount = 10
      expect(stateB.continuationCount).toBe(0)
    })
  })

  describe('resetState', () => {
    it('should remove the state for a project path', () => {
      const state1 = getState(projectPathA)
      state1.continuationCount = 5
      
      resetState(projectPathA)
      
      const state2 = getState(projectPathA)
      expect(state2).not.toBe(state1)
      expect(state2.continuationCount).toBe(0)
    })

    it('should do nothing if the project path does not exist', () => {
      expect(() => resetState('/non-existent')).not.toThrow()
    })
  })

  describe('updateState', () => {
    it('should update existing state with a patch', () => {
      const state = getState(projectPathA)
      const updated = updateState(projectPathA, {
        continuationCount: 3,
        inFlight: true
      })
      
      expect(updated).toBe(state)
      expect(state.continuationCount).toBe(3)
      expect(state.inFlight).toBe(true)
      expect(state.stagnationCount).toBe(0)
    })

    it('should ignore undefined values in patch', () => {
      const state = getState(projectPathA)
      state.continuationCount = 5
      
      updateState(projectPathA, {
        continuationCount: undefined,
        stagnationCount: 2
      })
      
      expect(state.continuationCount).toBe(5)
      expect(state.stagnationCount).toBe(2)
    })

    it('should handle optional fields like abortDetectedAt', () => {
      const now = Date.now()
      const state = updateState(projectPathA, { abortDetectedAt: now })
      
      expect(state.abortDetectedAt).toBe(now)
    })
  })

  describe('State Lifecycle', () => {
    it('should handle creation, mutation, reset, and re-creation', () => {
      const state1 = getState(projectPathA)
      expect(state1.continuationCount).toBe(0)
      
      updateState(projectPathA, { continuationCount: 1 })
      expect(state1.continuationCount).toBe(1)
      
      resetState(projectPathA)
      
      const state2 = getState(projectPathA)
      expect(state2).not.toBe(state1)
      expect(state2.continuationCount).toBe(0)
    })
  })
})
