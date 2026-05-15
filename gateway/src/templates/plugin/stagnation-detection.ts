import type { LuxSessionState } from "./session-state"
import type { Ticket } from "./ticket-loader"
import type { ExternalSignalIntegrator } from "./external-signal-integrator"

export const MAX_STAGNATION = 3
const HEALTH_SCORE_THRESHOLD = 30
const MAX_BUILD_FAILURE_STREAK = 3
const MAX_ZERO_PROGRESS_CYCLES = 5

export type StagnationReason =
  | "ticket_stagnation"
  | "health_degraded"
  | "build_failure_streak"
  | "zero_progress_cycle"

export interface StagnationDetails {
  shouldStop: boolean
  reasons: StagnationReason[]
  ticketStagnation: boolean
  healthScore?: number
  buildFailureStreak?: number
  zeroProgressCycles: number
}

function incompleteTicketCount(tickets: Ticket[]): number {
  return tickets.filter((ticket) => ticket.status !== "Done").length
}

function sessionWithZeroProgress(state: LuxSessionState): LuxSessionState & { zeroProgressCycles?: number } {
  return state as LuxSessionState & { zeroProgressCycles?: number }
}

function getZeroProgressCycles(state: LuxSessionState): number {
  return sessionWithZeroProgress(state).zeroProgressCycles ?? state.stagnationCount
}

function setZeroProgressCycles(state: LuxSessionState, value: number): void {
  ;(state as LuxSessionState & { zeroProgressCycles?: number }).zeroProgressCycles = value
}

function getBuildFailureStreak(integrator?: ExternalSignalIntegrator): number {
  if (!integrator) return 0

  const recentResults = integrator.getRecentResults()
  let streak = 0

  for (let index = recentResults.length - 1; index >= 0; index -= 1) {
    const result = recentResults[index]
    if (result.type === "build" && !result.success) {
      streak += 1
      continue
    }

    break
  }

  return streak
}

function getStagnationReasons(
  state: LuxSessionState,
  integrator?: ExternalSignalIntegrator,
  maxStagnation = MAX_STAGNATION,
): StagnationDetails {
  const zeroProgressCycles = getZeroProgressCycles(state)
  const reasons: StagnationReason[] = []
  const ticketStagnation = state.stagnationCount >= maxStagnation

  if (ticketStagnation) {
    reasons.push("ticket_stagnation")
  }

  const healthScore = integrator?.getHealthScore()
  if (typeof healthScore === "number" && healthScore < HEALTH_SCORE_THRESHOLD) {
    reasons.push("health_degraded")
  }

  const buildFailureStreak = getBuildFailureStreak(integrator)
  if (buildFailureStreak >= MAX_BUILD_FAILURE_STREAK) {
    reasons.push("build_failure_streak")
  }

  if (zeroProgressCycles >= MAX_ZERO_PROGRESS_CYCLES) {
    reasons.push("zero_progress_cycle")
  }

  return {
    shouldStop: reasons.length > 0,
    reasons,
    ticketStagnation,
    healthScore,
    buildFailureStreak,
    zeroProgressCycles,
  }
}

function logStagnationReasons(details: StagnationDetails): void {
  if (!details.shouldStop) return

  for (const reason of details.reasons) {
    if (reason === "ticket_stagnation") {
      console.warn("[stagnation] ticket_stagnation: incomplete tickets stopped decreasing")
    } else if (reason === "health_degraded") {
      console.warn(
        `[stagnation] health_degraded: health score ${details.healthScore ?? "unknown"} < ${HEALTH_SCORE_THRESHOLD}`,
      )
    } else if (reason === "build_failure_streak") {
      console.warn(
        `[stagnation] build_failure_streak: ${details.buildFailureStreak ?? 0} consecutive build failures >= ${MAX_BUILD_FAILURE_STREAK}`,
      )
    } else if (reason === "zero_progress_cycle") {
      console.warn(
        `[stagnation] zero_progress_cycle: ${details.zeroProgressCycles} zero-progress cycles >= ${MAX_ZERO_PROGRESS_CYCLES}`,
      )
    }
  }
}

export function trackProgress(
  state: LuxSessionState,
  currentTickets: Ticket[],
  currentAmbiguity: number,
  integrator?: ExternalSignalIntegrator,
): void {
  const currentIncomplete = incompleteTicketCount(currentTickets)
  const hadTicketBaseline = state.lastIncompleteTicketCount >= 0
  const hadAmbiguityBaseline = Number.isFinite(state.lastAmbiguityScore)
  const ticketProgressed = hadTicketBaseline && currentIncomplete < state.lastIncompleteTicketCount
  const ambiguityProgressed = hadAmbiguityBaseline && currentAmbiguity < state.lastAmbiguityScore
  const progressed = ticketProgressed || ambiguityProgressed

  if (progressed) {
    state.stagnationCount = 0
    setZeroProgressCycles(state, 0)
    state.awaitingPostInjectionProgressCheck = false
  } else if (state.awaitingPostInjectionProgressCheck) {
    state.stagnationCount += 1
    setZeroProgressCycles(state, getZeroProgressCycles(state) + 1)
    state.awaitingPostInjectionProgressCheck = false
  }

  if (integrator) {
    const recentResults = integrator.getRecentResults()
    let consecutiveBuildFailures = 0

    for (let index = recentResults.length - 1; index >= 0; index -= 1) {
      const result = recentResults[index]
      if (result.type === "build" && !result.success) {
        consecutiveBuildFailures += 1
        continue
      }

      break
    }

    state.consecutiveFailures = consecutiveBuildFailures
  }

  state.lastIncompleteTicketCount = currentIncomplete
  state.lastAmbiguityScore = currentAmbiguity
  setZeroProgressCycles(state, getZeroProgressCycles(state))
}

export function getStagnationDetails(
  state: LuxSessionState,
  integratorOrMaxStagnation?: ExternalSignalIntegrator | number,
  maxStagnation = MAX_STAGNATION,
): StagnationDetails {
  if (typeof integratorOrMaxStagnation === "number") {
    return getStagnationReasons(state, undefined, integratorOrMaxStagnation)
  }

  return getStagnationReasons(state, integratorOrMaxStagnation, maxStagnation)
}

export function shouldStopForStagnation(
  state: LuxSessionState,
  maxStagnationOrIntegrator?: number | ExternalSignalIntegrator,
  integrator?: ExternalSignalIntegrator,
): boolean {
  const details =
    typeof maxStagnationOrIntegrator === "number"
      ? getStagnationDetails(state, integrator, maxStagnationOrIntegrator)
      : getStagnationDetails(state, maxStagnationOrIntegrator)
  logStagnationReasons(details)
  return details.shouldStop
}
