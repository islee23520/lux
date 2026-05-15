import { describe, it, expect } from "vitest"
import {
  evaluateMaxContinuationsStop,
  evaluateUserAbortStop,
  evaluateStagnationStop,
  evaluateHealthCriticalStop,
  evaluateAllCompleteStop,
  evaluateAmbiguityTooHighStop,
  evaluateConsecutiveStateErrorStop,
  evaluateStopConditions,
  getStopReasonMessage,
  DEFAULT_STOP_CONFIG,
} from "../stop-evaluator"
import type { LuxSessionState } from "../session-state"
import type { StagnationDetails } from "../stagnation-detection"

describe("stop-evaluator", () => {
  const mockState: LuxSessionState = {
    continuationCount: 0,
    consecutiveFailures: 0,
    consecutiveCompileFailures: 0,
    stagnationCount: 0,
    lastIncompleteTicketCount: -1,
    lastAmbiguityScore: NaN,
    inFlight: false,
    lastInjectedAt: 0,
    awaitingPostInjectionProgressCheck: false,
    recentCompactionAt: null,
    recentCompactionEpoch: 0,
    acknowledgedCompactionEpoch: 0,
  }

  const mockStagnation: StagnationDetails = {
    shouldStop: false,
    reasons: [],
    ticketStagnation: false,
    zeroProgressCycles: 0,
  }

  describe("evaluateMaxContinuationsStop", () => {
    it("should stop when continuation count reaches max", () => {
      const state = { ...mockState, continuationCount: 50 }
      const result = evaluateMaxContinuationsStop(state, DEFAULT_STOP_CONFIG)
      expect(result?.shouldStop).toBe(true)
      expect(result?.reason).toBe("max_continations")
    })

    it("should respect custom max continuation config", () => {
      const state = { ...mockState, continuationCount: 2 }
      const result = evaluateMaxContinuationsStop(state, { ...DEFAULT_STOP_CONFIG, maxContinuations: 2 })
      expect(result?.reason).toBe("max_continations")
    })

    it("should not stop when below max", () => {
      const state = { ...mockState, continuationCount: 49 }
      const result = evaluateMaxContinuationsStop(state, DEFAULT_STOP_CONFIG)
      expect(result).toBeNull()
    })
  })

  describe("evaluateUserAbortStop", () => {
    it("should stop on abort detection", () => {
      const state = { ...mockState, abortDetectedAt: Date.now() }
      const result = evaluateUserAbortStop(state)
      expect(result?.reason).toBe("user_abort")
    })

    it("should stop on token limit detection", () => {
      const state = { ...mockState, tokenLimitDetected: true }
      const result = evaluateUserAbortStop(state)
      expect(result?.reason).toBe("user_abort")
    })

    it("should return null when no abort indicators are present", () => {
      expect(evaluateUserAbortStop(mockState)).toBeNull()
    })

    it("should include token limit in user abort message", () => {
      const context = {
        state: { ...mockState, tokenLimitDetected: true },
        stagnationDetails: mockStagnation,
        activeTickets: [{ status: "Todo" }],
        ambiguity: 0.1,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }

      expect(getStopReasonMessage("user_abort", context)).toContain("tokenLimitDetected=true")
    })
  })

  describe("evaluateStagnationStop", () => {
    it("should stop when stagnation details signal stop", () => {
      const details: StagnationDetails = { ...mockStagnation, shouldStop: true }
      const result = evaluateStagnationStop(details)
      expect(result?.reason).toBe("stagnation")
    })
  })

  describe("evaluateHealthCriticalStop", () => {
    it("should stop when health is low and failures are high", () => {
      const state = { ...mockState, consecutiveFailures: 3 }
      const result = evaluateHealthCriticalStop(state, 10, DEFAULT_STOP_CONFIG)
      expect(result?.reason).toBe("health_critical")
    })

    it("should honor custom threshold config", () => {
      const state = { ...mockState, consecutiveFailures: 1 }
      const result = evaluateHealthCriticalStop(state, 10, { ...DEFAULT_STOP_CONFIG, consecutiveFailuresThreshold: 1 })
      expect(result?.reason).toBe("health_critical")
    })

    it("should not stop when health is high", () => {
      const state = { ...mockState, consecutiveFailures: 3 }
      const result = evaluateHealthCriticalStop(state, 90, DEFAULT_STOP_CONFIG)
      expect(result).toBeNull()
    })
  })

  describe("evaluateAllCompleteStop", () => {
    it("should stop when no active tickets remain", () => {
      const result = evaluateAllCompleteStop([])
      expect(result?.reason).toBe("all_complete")
    })

    it("should not stop when tickets are active", () => {
      const result = evaluateAllCompleteStop([{ status: "Todo" }])
      expect(result).toBeNull()
    })

    it("should treat empty array as complete", () => {
      expect(evaluateAllCompleteStop([])?.confidence).toBe(0.99)
    })
  })

  describe("evaluateAmbiguityTooHighStop", () => {
    it("should stop when ambiguity is high and no clarification in progress", () => {
      const result = evaluateAmbiguityTooHighStop(0.95, false, DEFAULT_STOP_CONFIG)
      expect(result?.reason).toBe("ambiguity_too_high")
    })

    it("should respect custom ambiguity threshold", () => {
      const result = evaluateAmbiguityTooHighStop(0.5, false, { ...DEFAULT_STOP_CONFIG, ambiguityThreshold: 0.4 })
      expect(result?.reason).toBe("ambiguity_too_high")
    })

    it("should not stop when clarification is in progress", () => {
      const result = evaluateAmbiguityTooHighStop(0.95, true, DEFAULT_STOP_CONFIG)
      expect(result).toBeNull()
    })
  })

  describe("evaluateConsecutiveStateErrorStop", () => {
    it("should stop when state is Error and failures exceed threshold", () => {
      const result = evaluateConsecutiveStateErrorStop(
        { status: "Error", consecutive_failures: 3 },
        DEFAULT_STOP_CONFIG
      )
      expect(result?.reason).toBe("consecutive_state_error")
    })

    it("should return null when failure threshold is not met", () => {
      expect(evaluateConsecutiveStateErrorStop({ status: "Error", consecutive_failures: 1 }, DEFAULT_STOP_CONFIG)).toBeNull()
    })
  })

  describe("evaluateStopConditions", () => {
    it("should return first matching stop condition", () => {
      const context = {
        state: { ...mockState, continuationCount: 50 },
        stagnationDetails: mockStagnation,
        activeTickets: [{ status: "Todo" }],
        ambiguity: 0.1,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }
      const result = evaluateStopConditions(context)
      expect(result.shouldStop).toBe(true)
      expect(result.reason).toBe("max_continations")
    })

    it("should fall through to ambiguity stop when earlier checks do not match", () => {
      const context = {
        state: mockState,
        stagnationDetails: mockStagnation,
        activeTickets: [],
        ambiguity: 0.95,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }
      const result = evaluateStopConditions(context)
      expect(result.reason).toBe("all_complete")
    })

    it("should return noStopDecision if no conditions met", () => {
      const context = {
        state: mockState,
        stagnationDetails: mockStagnation,
        activeTickets: [{ status: "Todo" }],
        ambiguity: 0.1,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }
      const result = evaluateStopConditions(context)
      expect(result.shouldStop).toBe(false)
      expect(result.reason).toBeNull()
    })

    it("should fall through to ambiguity stop if only ambiguity is high", () => {
      const context = {
        state: mockState,
        stagnationDetails: mockStagnation,
        activeTickets: [{ status: "Todo" }],
        ambiguity: 0.95,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }
      const result = evaluateStopConditions(context)
      expect(result.reason).toBe("ambiguity_too_high")
    })
  })

  describe("getStopReasonMessage", () => {
    it("should format messages for all reasons", () => {
      const context = {
        state: mockState,
        stagnationDetails: { ...mockStagnation, reasons: ["health_degraded"] as any },
        activeTickets: [],
        ambiguity: 0.95,
        continuationState: { status: "Error", consecutive_failures: 3 },
        clarificationTicketInProgress: false,
      }

      expect(getStopReasonMessage("max_continations", context)).toContain("max_continations")
      expect(getStopReasonMessage("user_abort", context)).toContain("user_abort")
      expect(getStopReasonMessage("stagnation", context)).toContain("health_degraded")
      expect(getStopReasonMessage("health_critical", context)).toContain("health_critical")
      expect(getStopReasonMessage("all_complete", context)).toContain("all_complete")
      expect(getStopReasonMessage("ambiguity_too_high", context)).toContain("ambiguity_too_high")
      expect(getStopReasonMessage("consecutive_state_error", context)).toContain("consecutive_state_error")
    })

    it("should format health critical details", () => {
      const context = {
        state: { ...mockState, consecutiveFailures: 3 },
        stagnationDetails: { ...mockStagnation, healthScore: 10 },
        activeTickets: [],
        ambiguity: 0.1,
        continuationState: { status: "Active", consecutive_failures: 0 },
        clarificationTicketInProgress: false,
      }

      expect(getStopReasonMessage("health_critical", context)).toContain("healthScore 10")
    })
  })
})
