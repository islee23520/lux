export type RecoveryAction = "Retry" | "Abort" | "Backoff" | "Skip"

export type ErrorType =
  | "rate_limit"
  | "auth"
  | "network"
  | "invalid_session"
  | "timeout"
  | "unknown"

export interface ErrorClassification {
  errorType: ErrorType
  message: string
  status: number | null
}

export interface RecoveryDecision {
  action: RecoveryAction
  delayMs: number
  errorType: ErrorType
}

export interface RecoveryState {
  consecutiveErrorsByType: Record<ErrorType, number>
  lastErrorTime: string | null
  lastErrorType: ErrorType | null
}

function createEmptyCounts(): Record<ErrorType, number> {
  return {
    rate_limit: 0,
    auth: 0,
    network: 0,
    invalid_session: 0,
    timeout: 0,
    unknown: 0,
  }
}

function normalizeMessage(error: unknown): { message: string; status: number | null } {
  if (typeof error === "string") {
    return { message: error, status: null }
  }

  if (error instanceof Error) {
    const status = typeof (error as Error & { status?: unknown }).status === "number" ? (error as Error & { status?: number }).status ?? null : null
    return { message: error.message || String(error), status }
  }

  if (error && typeof error === "object") {
    const candidate = error as { message?: unknown; status?: unknown; statusCode?: unknown; code?: unknown }
    const status = typeof candidate.status === "number"
      ? candidate.status
      : typeof candidate.statusCode === "number"
        ? candidate.statusCode
        : null
    const parts = [candidate.message, candidate.code]
      .filter((value): value is string => typeof value === "string" && value.length > 0)
    return { message: parts.join(" ") || JSON.stringify(error), status }
  }

  return { message: String(error), status: null }
}

function includesAny(message: string, terms: string[]): boolean {
  return terms.some((term) => message.includes(term))
}

export function classifyError(error: unknown): ErrorClassification {
  const { message, status } = normalizeMessage(error)
  const normalized = message.toLowerCase()

  if (status === 429 || includesAny(normalized, ["rate limit", "ratelimit", "too many requests", "429", "rate"])) {
    return { errorType: "rate_limit", message, status }
  }

  if (status === 401 || status === 403 || includesAny(normalized, ["auth", "unauthorized", "forbidden"])) {
    return { errorType: "auth", message, status }
  }

  if (includesAny(normalized, ["session not found", "invalid session", "session missing"])) {
    return { errorType: "invalid_session", message, status }
  }

  if (includesAny(normalized, ["timeout", "aborted", "aborterror", "timed out"])) {
    return { errorType: "timeout", message, status }
  }

  if (
    includesAny(normalized, ["econnrefused", "enotfound", "network", "fetch failed", "econnreset", "etimedout"]) ||
    status === 503 ||
    status === 504
  ) {
    return { errorType: "network", message, status }
  }

  return { errorType: "unknown", message, status }
}

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value))
}

export function getBackoffDelayMs(errorType: ErrorType, consecutiveCount: number): number {
  const n = Math.max(1, consecutiveCount)

  if (errorType === "rate_limit") {
    return clamp(30000 * 2 ** (n - 1), 30000, 120000)
  }

  if (errorType === "network") {
    if (n <= 3) {
      return 0
    }
    return clamp(5000 * 2 ** (n - 3), 5000, 60000)
  }

  return clamp(5000 * 2 ** (n - 1), 5000, 60000)
}

export function handleRateLimit(consecutiveCount: number): number {
  return getBackoffDelayMs("rate_limit", consecutiveCount)
}

export function createRecoveryState(): RecoveryState {
  return {
    consecutiveErrorsByType: createEmptyCounts(),
    lastErrorTime: null,
    lastErrorType: null,
  }
}

function updateRecoveryState(state: RecoveryState, errorType: ErrorType): number {
  state.consecutiveErrorsByType[errorType] += 1
  state.lastErrorType = errorType
  state.lastErrorTime = new Date().toISOString()
  return state.consecutiveErrorsByType[errorType]
}

export function handlePromptAsyncError(
  error: unknown,
  state: RecoveryState = createRecoveryState(),
): RecoveryDecision {
  const classification = classifyError(error)
  const consecutiveCount = updateRecoveryState(state, classification.errorType)

  switch (classification.errorType) {
    case "rate_limit":
      return {
        action: "Backoff",
        delayMs: handleRateLimit(consecutiveCount),
        errorType: classification.errorType,
      }
    case "auth":
    case "invalid_session":
      return { action: "Abort", delayMs: 0, errorType: classification.errorType }
    case "timeout":
      return { action: "Skip", delayMs: 0, errorType: classification.errorType }
    case "network":
      if (consecutiveCount <= 3) {
        return { action: "Retry", delayMs: 0, errorType: classification.errorType }
      }
      return {
        action: "Backoff",
        delayMs: getBackoffDelayMs("network", consecutiveCount),
        errorType: classification.errorType,
      }
    case "unknown":
    default:
      if (consecutiveCount <= 1) {
        return { action: "Retry", delayMs: 0, errorType: classification.errorType }
      }
      return {
        action: "Backoff",
        delayMs: getBackoffDelayMs("unknown", consecutiveCount),
        errorType: classification.errorType,
      }
  }
}
