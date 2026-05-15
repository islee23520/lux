import { describe, it, expect } from "vitest"
import { getStagnationDetails, trackProgress } from "../stagnation-detection"
import { evaluateStopConditions } from "../stop-evaluator"
import type { LuxSessionState } from "../session-state"

describe("stagnation-stop-integration", () => {
  const createInitialState = (): LuxSessionState => ({
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
  })

  it("should propagate stagnation to stop evaluation", () => {
    const state = createInitialState()
    
    for (let i = 0; i < 3; i++) {
      state.awaitingPostInjectionProgressCheck = true
      trackProgress(state, [{ id: "1", status: "Todo", title: "Task" }], 0.5)
    }

    expect(state.stagnationCount).toBe(3)

    const stagnationDetails = getStagnationDetails(state)
    expect(stagnationDetails.shouldStop).toBe(true)
    expect(stagnationDetails.reasons).toContain("ticket_stagnation")

    const stopDecision = evaluateStopConditions({
      state,
      stagnationDetails,
      activeTickets: [{ status: "Todo" }],
      ambiguity: 0.5,
      continuationState: { status: "Active", consecutive_failures: 0 },
      clarificationTicketInProgress: false,
    })

    expect(stopDecision.shouldStop).toBe(true)
    expect(stopDecision.reason).toBe("stagnation")
  })

  it("should reset stagnation and allow continuation when progress is made", () => {
    const state = createInitialState()
    
    state.awaitingPostInjectionProgressCheck = true
    trackProgress(state, [{ id: "1", status: "Todo", title: "Task" }], 0.5)
    expect(state.stagnationCount).toBe(1)

    trackProgress(state, [], 0.5)
    expect(state.stagnationCount).toBe(0)

    const stagnationDetails = getStagnationDetails(state)
    const stopDecision = evaluateStopConditions({
      state,
      stagnationDetails,
      activeTickets: [],
      ambiguity: 0.5,
      continuationState: { status: "Active", consecutive_failures: 0 },
      clarificationTicketInProgress: false,
    })

    if (stopDecision.reason === "all_complete") {
        expect(stopDecision.shouldStop).toBe(true)
    } else {
        expect(stopDecision.shouldStop).toBe(false)
    }
  })

  it("should stop when health score drops below threshold", () => {
    const state = createInitialState()
    state.consecutiveFailures = 3

    const mockIntegrator = {
      getHealthScore: () => 10,
      getRecentResults: () => [],
    } as any

    const stagnationDetails = getStagnationDetails(state, mockIntegrator)
    expect(stagnationDetails.reasons).toContain("health_degraded")

    const stopDecision = evaluateStopConditions({
      state,
      stagnationDetails,
      activeTickets: [{ status: "Todo" }],
      ambiguity: 0.1,
      continuationState: { status: "Active", consecutive_failures: 0 },
      clarificationTicketInProgress: false,
    })

    expect(stopDecision.shouldStop).toBe(true)
    expect(stopDecision.reason).toBe("health_critical")
  })
})
