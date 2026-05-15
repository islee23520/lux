import { describe, it, expect, beforeEach } from "vitest"
import { 
  armCompactionGuard, 
  isCompactionGuardActive, 
  acknowledgeCompaction,
  COMPACTION_GUARD_MS 
} from "../compaction-guard"
import type { LuxSessionState } from "../session-state"

describe("compaction-guard", () => {
  let state: LuxSessionState

  beforeEach(() => {
    state = {
      continuationCount: 0,
      lastInjectedAt: 0,
      awaitingPostInjectionProgressCheck: false,
      inFlight: false,
      stagnationCount: 0,
      consecutiveFailures: 0,
      consecutiveCompileFailures: 0,
      recentCompactionAt: null,
      recentCompactionEpoch: 0,
      acknowledgedCompactionEpoch: 0,
      lastIncompleteTicketCount: -1,
      lastAmbiguityScore: Number.POSITIVE_INFINITY,
    }
  })

  describe("armCompactionGuard", () => {
    it("sets recentCompactionAt and increments recentCompactionEpoch", () => {
      const now = 1000
      armCompactionGuard(state, now)
      
      expect(state.recentCompactionAt).toBe(now)
      expect(state.recentCompactionEpoch).toBe(1)
      expect(state.acknowledgedCompactionEpoch).toBe(0)
    })

    it("resets acknowledgedCompactionEpoch to 0", () => {
      state.acknowledgedCompactionEpoch = 5
      armCompactionGuard(state, 2000)
      expect(state.acknowledgedCompactionEpoch).toBe(0)
    })

    it("increments epoch on multiple calls (idempotency check)", () => {
      armCompactionGuard(state, 1000)
      expect(state.recentCompactionEpoch).toBe(1)
      
      armCompactionGuard(state, 1100)
      expect(state.recentCompactionEpoch).toBe(2)
    })
  })

  describe("isCompactionGuardActive", () => {
    it("returns false if recentCompactionAt is null", () => {
      expect(isCompactionGuardActive(state, 1000)).toBe(false)
    })

    it("returns true immediately after arming", () => {
      const now = 1000
      armCompactionGuard(state, now)
      expect(isCompactionGuardActive(state, now)).toBe(true)
    })

    it("returns true within the guard window", () => {
      const armedAt = 1000
      armCompactionGuard(state, armedAt)
      
      expect(isCompactionGuardActive(state, armedAt + COMPACTION_GUARD_MS - 1)).toBe(true)
    })

    it("returns false exactly at the threshold", () => {
      const armedAt = 1000
      armCompactionGuard(state, armedAt)
      
      expect(isCompactionGuardActive(state, armedAt + COMPACTION_GUARD_MS)).toBe(false)
    })

    it("returns false after the guard window", () => {
      const armedAt = 1000
      armCompactionGuard(state, armedAt)
      
      expect(isCompactionGuardActive(state, armedAt + COMPACTION_GUARD_MS + 1)).toBe(false)
    })

    it("returns false if already acknowledged for the current epoch", () => {
      const armedAt = 1000
      armCompactionGuard(state, armedAt)
      
      acknowledgeCompaction(state)
      expect(isCompactionGuardActive(state, armedAt + 100)).toBe(false)
    })

    it("returns true if armed again after acknowledgment", () => {
      armCompactionGuard(state, 1000)
      acknowledgeCompaction(state)
      expect(isCompactionGuardActive(state, 1100)).toBe(false)
      
      armCompactionGuard(state, 2000)
      expect(isCompactionGuardActive(state, 2100)).toBe(true)
    })
  })

  describe("acknowledgeCompaction", () => {
    it("sets acknowledgedCompactionEpoch to the current recentCompactionEpoch by default", () => {
      state.recentCompactionEpoch = 3
      acknowledgeCompaction(state)
      expect(state.acknowledgedCompactionEpoch).toBe(3)
    })

    it("can acknowledge a specific epoch", () => {
      state.recentCompactionEpoch = 5
      acknowledgeCompaction(state, 4)
      expect(state.acknowledgedCompactionEpoch).toBe(4)
      state.recentCompactionAt = 1000
      expect(isCompactionGuardActive(state, 1100)).toBe(true)
    })
  })

  describe("complex cycles", () => {
    it("handles multiple arm/ack cycles correctly", () => {
      armCompactionGuard(state, 1000)
      expect(isCompactionGuardActive(state, 1100)).toBe(true)
      acknowledgeCompaction(state)
      expect(isCompactionGuardActive(state, 1200)).toBe(false)

      armCompactionGuard(state, 5000)
      expect(isCompactionGuardActive(state, 5100)).toBe(true)
      
      expect(isCompactionGuardActive(state, 5000 + COMPACTION_GUARD_MS + 1)).toBe(false)
      
      acknowledgeCompaction(state)
      expect(isCompactionGuardActive(state, 5200)).toBe(false)
    })
  })
})
