import type { StagnationDetails } from "./stagnation-detection"
import type { LuxSessionState } from "./session-state"

export type StopReason =
  | "max_continuations_reached"
  | "user_abort"
  | "stagnation"
  | "health_critical"
  | "all_complete"
  | "ambiguity_too_high"
  | "consecutive_state_error"

export interface StopDecision {
  shouldStop: boolean
  reason: StopReason | null
  confidence: number
}

export interface StopConfig {
  maxContinuations: number
  healthThreshold: number
  consecutiveFailuresThreshold: number
  ambiguityThreshold: number
}

export const DEFAULT_STOP_CONFIG: StopConfig = {
  maxContinuations: 50,
  healthThreshold: 20,
  consecutiveFailuresThreshold: 3,
  ambiguityThreshold: 0.9,
}

export interface StopTicketLike {
  status: string
  title?: string | null
}

export interface StopContinuationState {
  status: string
  consecutive_failures: number
}

export interface StopEvaluationContext {
  state: LuxSessionState
  stagnationDetails: StagnationDetails
  activeTickets: ReadonlyArray<StopTicketLike>
  ambiguity: number
  continuationState: StopContinuationState
  clarificationTicketInProgress: boolean
  config?: Partial<StopConfig>
}

function resolveConfig(config?: Partial<StopConfig>): StopConfig {
  return {
    ...DEFAULT_STOP_CONFIG,
    ...config,
  }
}

function stopDecision(reason: StopReason, confidence: number): StopDecision {
  return {
    shouldStop: true,
    reason,
    confidence,
  }
}

function noStopDecision(): StopDecision {
  return {
    shouldStop: false,
    reason: null,
    confidence: 0,
  }
}

/**
 * Triggers when the session has reached the configured continuation ceiling.
 */
export function evaluateMaxContinuationsStop(
  state: LuxSessionState,
  config: StopConfig,
): StopDecision | null {
  return state.continuationCount >= config.maxContinuations ? stopDecision("max_continuations_reached", 1) : null
}

/**
 * Triggers when an abort was detected or the model signaled token exhaustion.
 */
export function evaluateUserAbortStop(state: LuxSessionState): StopDecision | null {
  return state.abortDetectedAt !== undefined || state.tokenLimitDetected === true
    ? stopDecision("user_abort", 1)
    : null
}

/**
 * Triggers when the enhanced stagnation detector marks the session as stop-worthy.
 */
export function evaluateStagnationStop(details: StagnationDetails): StopDecision | null {
  return details.shouldStop ? stopDecision("stagnation", 0.92) : null
}

/**
 * Triggers when the health score is below threshold and the session has failed repeatedly.
 */
export function evaluateHealthCriticalStop(
  state: LuxSessionState,
  healthScore: number | undefined,
  config: StopConfig,
): StopDecision | null {
  return typeof healthScore === "number" && healthScore < config.healthThreshold && state.consecutiveFailures >= config.consecutiveFailuresThreshold
    ? stopDecision("health_critical", 0.98)
    : null
}

/**
 * Triggers when no active tickets remain to drive the continuation loop.
 */
export function evaluateAllCompleteStop(activeTickets: ReadonlyArray<StopTicketLike>): StopDecision | null {
  return activeTickets.length === 0 ? stopDecision("all_complete", 0.99) : null
}

/**
 * Triggers when ambiguity is very high and there is no clarification work already in progress.
 */
export function evaluateAmbiguityTooHighStop(
  ambiguity: number,
  clarificationTicketInProgress: boolean,
  config: StopConfig,
): StopDecision | null {
  return ambiguity > config.ambiguityThreshold && !clarificationTicketInProgress
    ? stopDecision("ambiguity_too_high", 0.88)
    : null
}

/**
 * Triggers when the persisted continuation state has entered Error and failure streak is too high.
 */
export function evaluateConsecutiveStateErrorStop(
  continuationState: StopContinuationState,
  config: StopConfig,
): StopDecision | null {
  return continuationState.status === "Error" && continuationState.consecutive_failures >= config.consecutiveFailuresThreshold
    ? stopDecision("consecutive_state_error", 0.97)
    : null
}

export function evaluateStopConditions(context: StopEvaluationContext): StopDecision {
  const config = resolveConfig(context.config)

  return (
    evaluateMaxContinuationsStop(context.state, config) ??
    evaluateUserAbortStop(context.state) ??
    evaluateConsecutiveStateErrorStop(context.continuationState, config) ??
    evaluateHealthCriticalStop(context.state, context.stagnationDetails.healthScore, config) ??
    evaluateStagnationStop(context.stagnationDetails) ??
    evaluateAllCompleteStop(context.activeTickets) ??
    evaluateAmbiguityTooHighStop(context.ambiguity, context.clarificationTicketInProgress, config) ??
    noStopDecision()
  )
}

export function getStopReasonMessage(
  reason: StopReason,
  context: Pick<StopEvaluationContext, "state" | "stagnationDetails" | "activeTickets" | "ambiguity" | "continuationState" | "clarificationTicketInProgress"> & {
    config?: Partial<StopConfig>
  },
): string {
  const config = resolveConfig(context.config)

  switch (reason) {
    case "max_continuations_reached":
      return `[stop-evaluator] max_continuations_reached: continuationCount ${context.state.continuationCount} >= ${config.maxContinuations}`
    case "user_abort":
      return `[stop-evaluator] user_abort: abortDetectedAt=${context.state.abortDetectedAt ?? "none"}, tokenLimitDetected=${context.state.tokenLimitDetected === true}`
    case "stagnation":
      return `[stop-evaluator] stagnation: ${context.stagnationDetails.reasons.join(",") || "stop-worthy stagnation detected"}`
    case "health_critical":
      return `[stop-evaluator] health_critical: healthScore ${context.stagnationDetails.healthScore ?? "unknown"} < ${config.healthThreshold}, consecutiveFailures ${context.state.consecutiveFailures} >= ${config.consecutiveFailuresThreshold}`
    case "all_complete":
      return `[stop-evaluator] all_complete: activeTickets=${context.activeTickets.length}`
    case "ambiguity_too_high":
      return `[stop-evaluator] ambiguity_too_high: ambiguity ${context.ambiguity} > ${config.ambiguityThreshold}, clarificationTicketInProgress=${context.clarificationTicketInProgress}`
    case "consecutive_state_error":
      return `[stop-evaluator] consecutive_state_error: continuationState.status=${context.continuationState.status}, consecutive_failures=${context.continuationState.consecutive_failures} >= ${config.consecutiveFailuresThreshold}`
  }
}
