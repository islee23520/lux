import { describe, it, expect, beforeEach } from "vitest"
import {
  classifyError,
  getBackoffDelayMs,
  handlePromptAsyncError,
  createRecoveryState,
  handleRateLimit,
} from "../error-recovery"

describe("error-recovery", () => {
  describe("classifyError", () => {
    it("classifies 429 status as rate_limit", () => {
      const error = { status: 429, message: "Too many requests" }
      const result = classifyError(error)
      expect(result.errorType).toBe("rate_limit")
      expect(result.status).toBe(429)
    })

    it("classifies string status-like payloads as rate limit keywords", () => {
      expect(classifyError("429 too many requests").errorType).toBe("rate_limit")
    })

    it("classifies object code/status combinations as auth or network", () => {
      expect(classifyError({ status: 401, code: "EUNAUTHORIZED", message: "x" }).errorType).toBe("auth")
      expect(classifyError({ status: 503, message: "fetch failed" }).errorType).toBe("network")
    })

    it("classifies rate limit keywords as rate_limit", () => {
      expect(classifyError("Rate limit exceeded").errorType).toBe("rate_limit")
      expect(classifyError("ratelimit").errorType).toBe("rate_limit")
      expect(classifyError("too many requests").errorType).toBe("rate_limit")
    })

    it("classifies 401/403 status as auth", () => {
      expect(classifyError({ status: 401 }).errorType).toBe("auth")
      expect(classifyError({ status: 403 }).errorType).toBe("auth")
    })

    it("classifies auth keywords as auth", () => {
      expect(classifyError("unauthorized").errorType).toBe("auth")
      expect(classifyError("forbidden").errorType).toBe("auth")
    })

    it("classifies session keywords as invalid_session", () => {
      expect(classifyError("session not found").errorType).toBe("invalid_session")
      expect(classifyError("invalid session").errorType).toBe("invalid_session")
    })

    it("classifies timeout keywords as timeout", () => {
      expect(classifyError("request timed out").errorType).toBe("timeout")
      expect(classifyError("aborted").errorType).toBe("timeout")
    })

    it("classifies network keywords and 503/504 as network", () => {
      expect(classifyError("ECONNREFUSED").errorType).toBe("network")
      expect(classifyError("fetch failed").errorType).toBe("network")
      expect(classifyError({ status: 503 }).errorType).toBe("network")
      expect(classifyError({ status: 504 }).errorType).toBe("network")
    })

    it("classifies unknown errors as unknown", () => {
      expect(classifyError("something went wrong").errorType).toBe("unknown")
      expect(classifyError({}).errorType).toBe("unknown")
      expect(classifyError(null).errorType).toBe("unknown")
    })

    it("prefers statusCode and code/message object fields", () => {
      const result = classifyError({ statusCode: 504, message: "fetch failed", code: "ETIMEDOUT" })
      expect(result.errorType).toBe("network")
      expect(result.status).toBe(504)
    })

    it("preserves string and Error message text", () => {
      const result1 = classifyError("plain failure")
      const result2 = classifyError(new Error("boom"))

      expect(result1.message).toBe("plain failure")
      expect(result2.message).toBe("boom")
    })

    it("classifies object string messages as unknown when no keywords match", () => {
      expect(classifyError({ message: "misc" }).errorType).toBe("unknown")
    })

    it("handles Error objects correctly", () => {
      const err = new Error("rate limit")
      // @ts-ignore
      err.status = 429
      const result = classifyError(err)
      expect(result.errorType).toBe("rate_limit")
      expect(result.status).toBe(429)
    })
  })

  describe("getBackoffDelayMs", () => {
    it("calculates exponential backoff for rate_limit", () => {
      expect(getBackoffDelayMs("rate_limit", 1)).toBe(30000)
      expect(getBackoffDelayMs("rate_limit", 2)).toBe(60000)
      expect(getBackoffDelayMs("rate_limit", 3)).toBe(120000)
      expect(getBackoffDelayMs("rate_limit", 4)).toBe(120000)
    })

    it("calculates backoff for network errors (0 for first 3)", () => {
      expect(getBackoffDelayMs("network", 1)).toBe(0)
      expect(getBackoffDelayMs("network", 2)).toBe(0)
      expect(getBackoffDelayMs("network", 3)).toBe(0)
      expect(getBackoffDelayMs("network", 4)).toBe(10000)
      expect(getBackoffDelayMs("network", 10)).toBe(60000)
    })

    it("calculates backoff for other errors", () => {
      expect(getBackoffDelayMs("unknown", 1)).toBe(5000)
      expect(getBackoffDelayMs("unknown", 2)).toBe(10000)
      expect(getBackoffDelayMs("unknown", 5)).toBe(60000)
    })
  })

  describe("handlePromptAsyncError", () => {
    let state: any

    beforeEach(() => {
      state = createRecoveryState()
    })

    it("returns Backoff for rate_limit", () => {
      const result = handlePromptAsyncError("rate limit", state)
      expect(result.action).toBe("Backoff")
      expect(result.delayMs).toBe(30000)
      expect(state.consecutiveErrorsByType.rate_limit).toBe(1)
    })

    it("returns Abort for auth and invalid_session", () => {
      expect(handlePromptAsyncError("unauthorized", state).action).toBe("Abort")
      expect(handlePromptAsyncError("session not found", state).action).toBe("Abort")
    })

    it("returns Skip for timeout", () => {
      expect(handlePromptAsyncError("timeout", state).action).toBe("Skip")
    })

    it("returns Retry for first 3 network errors, then Backoff", () => {
      expect(handlePromptAsyncError("network error", state).action).toBe("Retry")
      expect(handlePromptAsyncError("network error", state).action).toBe("Retry")
      expect(handlePromptAsyncError("network error", state).action).toBe("Retry")
      
      const result = handlePromptAsyncError("network error", state)
      expect(result.action).toBe("Backoff")
      expect(result.delayMs).toBe(10000)
    })

    it("returns Retry for first unknown error, then Backoff", () => {
      expect(handlePromptAsyncError("unknown", state).action).toBe("Retry")
      
      const result = handlePromptAsyncError("unknown", state)
      expect(result.action).toBe("Backoff")
      expect(result.delayMs).toBe(10000)
    })

    it("tracks state independently by error type", () => {
      const rateLimit = handlePromptAsyncError("rate limit", state)
      const auth = handlePromptAsyncError("unauthorized", state)

      expect(rateLimit.errorType).toBe("rate_limit")
      expect(auth.errorType).toBe("auth")
      expect(state.consecutiveErrorsByType.rate_limit).toBe(1)
      expect(state.consecutiveErrorsByType.auth).toBe(1)
    })

    it("returns abort for invalid session errors", () => {
      expect(handlePromptAsyncError("session not found", state).action).toBe("Abort")
    })

    it("returns network backoff after the fourth network error", () => {
      handlePromptAsyncError("network error", state)
      handlePromptAsyncError("network error", state)
      handlePromptAsyncError("network error", state)
      const result = handlePromptAsyncError("network error", state)

      expect(result.action).toBe("Backoff")
      expect(result.delayMs).toBeGreaterThan(0)
    })

    it("uses default state if not provided", () => {
      const result = handlePromptAsyncError("rate limit")
      expect(result.action).toBe("Backoff")
    })
  })

  describe("handleRateLimit", () => {
    it("is a wrapper around getBackoffDelayMs for rate_limit", () => {
      expect(handleRateLimit(1)).toBe(30000)
      expect(handleRateLimit(2)).toBe(60000)
    })
  })
})
