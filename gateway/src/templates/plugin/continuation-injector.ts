import type { LuxEvalResult, LuxPluginConfig } from "./types"
import type { LuxSessionState } from "./session-state"
import type { ContinuationState } from "./continuation-state-client"

export interface OpenCodePromptContext {
  directory: string
  client?: {
    session?: {
      promptAsync?: (input: {
        path: { id: string }
        body: { parts: Array<{ type: "text"; text: string }> }
        query: { directory: string }
      }) => Promise<unknown>
    }
    tui?: {
      appendPrompt?: (input: { body: { parts: Array<{ type: "text"; text: string }> } }) => Promise<unknown>
    }
  }
}

interface SessionState {
  continuationCount: number
  lastAction: string
  lastTimestamp: number
}

const DEFAULT_MAX_CONTINUATIONS = 50
const DEFAULT_TARGET_AMBIGUITY = 0.02
export const CONTINUATION_COOLDOWN_MS = 5_000
export const MAX_CONSECUTIVE_FAILURES = 5

const sessionStates = new Map<string, SessionState>()

function resolveMaxContinuations(config: LuxPluginConfig): number {
  return config.maxContinuations || DEFAULT_MAX_CONTINUATIONS
}

function resolveTargetAmbiguity(config: LuxPluginConfig): number {
  return config.targetAmbiguity || DEFAULT_TARGET_AMBIGUITY
}

function getOrCreateState(projectPath: string): SessionState {
  let state = sessionStates.get(projectPath)
  if (!state) {
    state = { continuationCount: 0, lastAction: "", lastTimestamp: Date.now() }
    sessionStates.set(projectPath, state)
  }
  return state
}

export function resetSession(projectPath: string): void {
  sessionStates.delete(projectPath)
}

export function getContinuationCount(projectPath: string, gatewayState?: ContinuationState): number {
  if (gatewayState !== undefined) {
    return gatewayState.continuation_count
  }
  return getOrCreateState(projectPath).continuationCount
}

export function canContinue(projectPath: string, config: LuxPluginConfig, gatewayState?: ContinuationState): boolean {
  const count = getContinuationCount(projectPath, gatewayState)
  return count < resolveMaxContinuations(config)
}

export function formatNextAction(evalResult: LuxEvalResult): string {
  if (!evalResult.should_continue) {
    return ""
  }

  const parts: string[] = []

  if (evalResult.ambiguity_score > 0.7) {
    parts.push("[Lux] Spec is highly ambiguous. Addressing critical gaps:")
  } else if (evalResult.ambiguity_score > 0.4) {
    parts.push("[Lux] Spec needs refinement. Next priority:")
  } else {
    parts.push("[Lux] Spec is nearly complete. Remaining item:")
  }

  if (evalResult.next_action) {
    parts.push(evalResult.next_action)
  }

  return parts.join(" ")
}

export function formatMaxReachedMessage(_projectPath: string, config: LuxPluginConfig): string {
  const maxContinuations = resolveMaxContinuations(config)
  return `[Lux] Maximum continuations reached (${maxContinuations}). Current spec ambiguity: review and update manually, or start a new session to continue.`
}

export function decideContinuation(
  projectPath: string,
  evalResult: LuxEvalResult,
  config: LuxPluginConfig,
): {
  shouldInject: boolean
  message: string
  continuationCount: number
} {
  const state = getOrCreateState(projectPath)

  if (evalResult.ambiguity_score <= resolveTargetAmbiguity(config)) {
    return {
      shouldInject: false,
      message: "",
      continuationCount: state.continuationCount,
    }
  }

  if (!evalResult.should_continue) {
    return {
      shouldInject: false,
      message: "",
      continuationCount: state.continuationCount,
    }
  }

  if (state.continuationCount >= resolveMaxContinuations(config)) {
    return {
      shouldInject: false,
      message: formatMaxReachedMessage(projectPath, config),
      continuationCount: state.continuationCount,
    }
  }

  state.continuationCount += 1
  state.lastAction = evalResult.next_action
  state.lastTimestamp = Date.now()

  return {
    shouldInject: true,
    message: formatNextAction(evalResult),
    continuationCount: state.continuationCount,
  }
}

export function getSessionSummary(projectPath: string): {
  continuationCount: number
  lastAction: string
  elapsedMs: number
} {
  const state = getOrCreateState(projectPath)
  return {
    continuationCount: state.continuationCount,
    lastAction: state.lastAction,
    elapsedMs: Date.now() - state.lastTimestamp,
  }
}

export function canInjectContinuation(state: LuxSessionState, now = Date.now()): boolean {
  if (state.inFlight) return false
  if (state.consecutiveFailures >= MAX_CONSECUTIVE_FAILURES) return false
  return now - state.lastInjectedAt >= CONTINUATION_COOLDOWN_MS
}

function buildPromptParts(message: string): Array<{ type: "text"; text: string }> {
  return [{ type: "text", text: message }]
}

export async function injectContinuation(args: {
  ctx: OpenCodePromptContext
  sessionID: string
  message: string
  state: LuxSessionState
}): Promise<boolean> {
  const { ctx, sessionID, message, state } = args
  if (!canInjectContinuation(state)) return false

  const promptAsync = ctx.client?.session?.promptAsync
  if (!promptAsync) {
    state.lastInjectedAt = Date.now()
    state.consecutiveFailures += 1
    return false
  }

  state.inFlight = true
  try {
    await promptAsync({
      path: { id: sessionID },
      body: { parts: buildPromptParts(message) },
      query: { directory: ctx.directory },
    })
    state.inFlight = false
    state.lastInjectedAt = Date.now()
    state.awaitingPostInjectionProgressCheck = true
    state.consecutiveFailures = 0
    return true
  } catch {
    state.inFlight = false
    state.lastInjectedAt = Date.now()
    state.consecutiveFailures += 1
    return false
  }
}

export async function injectAppendPrompt(args: {
  ctx: OpenCodePromptContext
  message: string
}): Promise<boolean> {
  const { ctx, message } = args
  const appendPrompt = ctx.client?.tui?.appendPrompt
  if (!appendPrompt) return false

  await appendPrompt({ body: { parts: buildPromptParts(message) } })
  return true
}
