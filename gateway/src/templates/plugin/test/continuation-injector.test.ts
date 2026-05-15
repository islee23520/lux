import { describe, it, expect, vi, beforeEach } from "vitest"
import {
  canContinue,
  decideContinuation,
  formatNextAction,
  formatMaxReachedMessage,
  canInjectContinuation,
  getContinuationCount,
  getSessionSummary,
  injectContinuation,
  injectAppendPrompt,
  resetSession,
} from "../continuation-injector"
import type { LuxSessionState } from "../session-state"
import type { LuxEvalResult, LuxPluginConfig } from "../types"
import type { OpenCodePromptContext } from "../continuation-injector"

describe("continuation-injector", () => {
  const projectPath = "/test/project"
  const defaultConfig: LuxPluginConfig = {
    maxContinuations: 10,
    specPath: ".lux/spec.json",
    glossaryPath: ".lux/glossary.md",
    targetAmbiguity: 0.02,
  }

  beforeEach(() => {
    resetSession(projectPath)
  })

  const mockSessionState = (): LuxSessionState => ({
    continuationCount: 0,
    consecutiveFailures: 0,
    stagnationCount: 0,
    lastIncompleteTicketCount: -1,
    lastAmbiguityScore: NaN,
    consecutiveCompileFailures: 0,
    recentCompactionAt: null,
    recentCompactionEpoch: 0,
    acknowledgedCompactionEpoch: 0,
    inFlight: false,
    lastInjectedAt: 0,
    awaitingPostInjectionProgressCheck: false,
  })

  describe("canContinue", () => {
    it("should return true when count is below max", () => {
      expect(canContinue(projectPath, defaultConfig)).toBe(true)
    })

    it("should return false when count reaches max", () => {
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "test",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }
      
      for (let i = 0; i < 10; i++) {
        decideContinuation(projectPath, evalResult, defaultConfig)
      }
      
      expect(canContinue(projectPath, defaultConfig)).toBe(false)
    })
  })

  describe("formatNextAction", () => {
    it("should format high ambiguity message", () => {
      const result = formatNextAction({
        should_continue: true,
        next_action: "Fix it",
        ambiguity_score: 0.8,
        continuation_count: 0,
      })
      expect(result).toContain("highly ambiguous")
      expect(result).toContain("Fix it")
    })

    it("should return empty string if should_continue is false", () => {
      const result = formatNextAction({
        should_continue: false,
        next_action: "Fix it",
        ambiguity_score: 0.8,
        continuation_count: 0,
      })
      expect(result).toBe("")
    })

    it("should format low ambiguity continuation as nearly complete", () => {
      const result = formatNextAction({
        should_continue: true,
        next_action: "fix X",
        ambiguity_score: 0.2,
        continuation_count: 0,
      })

      expect(result).toContain("nearly complete")
      expect(result).toContain("fix X")
    })
  })

  describe("formatMaxReachedMessage", () => {
    it("should include configured max continuation count", () => {
      const result = formatMaxReachedMessage(projectPath, {
        ...defaultConfig,
        maxContinuations: 5,
      })

      expect(result).toContain("Maximum continuations reached")
      expect(result).toContain("(5)")
    })
  })

  describe("decideContinuation", () => {
    it("should increment count and return shouldInject: true", () => {
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Next step",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }
      
      const result = decideContinuation(projectPath, evalResult, defaultConfig)
      expect(result.shouldInject).toBe(true)
      expect(result.continuationCount).toBe(1)
      expect(result.message).toContain("Next step")
    })

    it("should return shouldInject: false if ambiguity is below target", () => {
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Next step",
        ambiguity_score: 0.01,
        continuation_count: 0,
      }
      
      const result = decideContinuation(projectPath, evalResult, defaultConfig)
      expect(result.shouldInject).toBe(false)
    })

    it("should expose an initially zero count and increment after continuation decision", () => {
      const countPath = `${projectPath}/count`
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Increase coverage",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }

      resetSession(countPath)
      expect(getContinuationCount(countPath)).toBe(0)

      decideContinuation(countPath, evalResult, defaultConfig)

      expect(getContinuationCount(countPath)).toBe(1)
    })

    it("should not inject when evaluation says not to continue", () => {
      const evalResult: LuxEvalResult = {
        should_continue: false,
        next_action: "Stop",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }

      const result = decideContinuation(projectPath, evalResult, defaultConfig)

      expect(result.shouldInject).toBe(false)
      expect(result.message).toBe("")
      expect(result.continuationCount).toBe(0)
    })

    it("should prefer the low-ambiguity branch and respect custom target ambiguity", () => {
      const lowResult = formatNextAction({
        should_continue: true,
        next_action: "Low branch",
        ambiguity_score: 0.3,
        continuation_count: 0,
      })
      const config: LuxPluginConfig = { ...defaultConfig, targetAmbiguity: 0.4, maxContinuations: 3 }
      const suppressed = decideContinuation(projectPath, {
        should_continue: true,
        next_action: "Custom target",
        ambiguity_score: 0.35,
        continuation_count: 0,
      }, config)

      expect(lowResult).toContain("Remaining item")
      expect(suppressed.shouldInject).toBe(false)
    })

    it("should fall back to defaults and block injection when failures are too high", () => {
      const config: LuxPluginConfig = { maxContinuations: 0, specPath: ".", glossaryPath: ".", targetAmbiguity: 0 }
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Fallback check",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }
      const state = mockSessionState()
      state.consecutiveFailures = 5
      state.lastInjectedAt = Date.now() - 10_000

      expect(formatNextAction({ ...evalResult, ambiguity_score: 0.2 })).toContain("Spec is nearly complete")
      expect(canContinue(projectPath, config)).toBe(true)
      expect(canInjectContinuation(state)).toBe(false)
    })

    it("should skip next-action text when none is provided", () => {
      const result = formatNextAction({
        should_continue: true,
        next_action: "",
        ambiguity_score: 0.8,
        continuation_count: 0,
      } as LuxEvalResult)

      expect(result).toContain("critical gaps")
      expect(result).not.toContain("undefined")
    })

    it("should stop injecting once max continuations is reached", () => {
      const maxPath = `${projectPath}/max`
      const config: LuxPluginConfig = { ...defaultConfig, maxContinuations: 2 }
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Keep going",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }

      resetSession(maxPath)
      expect(decideContinuation(maxPath, evalResult, config).shouldInject).toBe(true)
      expect(decideContinuation(maxPath, evalResult, config).shouldInject).toBe(true)

      const result = decideContinuation(maxPath, evalResult, config)

      expect(result.shouldInject).toBe(false)
      expect(result.message).toContain("Maximum")
      expect(result.continuationCount).toBe(2)
    })

    it("should return session summary with count, last action, and elapsed time", () => {
      const summaryPath = `${projectPath}/summary`
      const evalResult: LuxEvalResult = {
        should_continue: true,
        next_action: "Summarize progress",
        ambiguity_score: 0.5,
        continuation_count: 0,
      }

      resetSession(summaryPath)
      decideContinuation(summaryPath, evalResult, defaultConfig)


      const summary = getSessionSummary(summaryPath)

      expect(summary).toMatchObject({
        continuationCount: 1,
        lastAction: "Summarize progress",
      })
      expect(summary.elapsedMs).toBeGreaterThanOrEqual(0)
    })
  })

  describe("canInjectContinuation", () => {
    const mockState = (): LuxSessionState => ({
      continuationCount: 0,
      consecutiveFailures: 0,
      stagnationCount: 0,
      lastIncompleteTicketCount: -1,
      lastAmbiguityScore: NaN,
      consecutiveCompileFailures: 0,
      recentCompactionAt: null,
      recentCompactionEpoch: 0,
      acknowledgedCompactionEpoch: 0,
      inFlight: false,
      lastInjectedAt: 0,
      awaitingPostInjectionProgressCheck: false,
    })

    it("should return true if cooldown passed and not in flight", () => {
      const state = mockState()
      state.lastInjectedAt = Date.now() - 10000
      expect(canInjectContinuation(state)).toBe(true)
    })

    it("should return false if in flight", () => {
      const state = mockState()
      state.inFlight = true
      expect(canInjectContinuation(state)).toBe(false)
    })

    it("should return false if cooldown not passed", () => {
      const state = mockState()
      state.lastInjectedAt = Date.now() - 1000
      expect(canInjectContinuation(state)).toBe(false)
    })
  })

  describe("injectContinuation", () => {
    it("should call promptAsync and update state", async () => {
      const promptAsync = vi.fn().mockResolvedValue({})
      const ctx = {
        directory: "/test",
        client: {
          session: { promptAsync }
        }
      }
      const state = {
        continuationCount: 0,
        consecutiveFailures: 0,
        stagnationCount: 0,
        lastIncompleteTicketCount: -1,
        lastAmbiguityScore: NaN,
        consecutiveCompileFailures: 0,
        recentCompactionAt: null,
        recentCompactionEpoch: 0,
        acknowledgedCompactionEpoch: 0,
        inFlight: false,
        lastInjectedAt: 0,
        awaitingPostInjectionProgressCheck: false,
      }

      const result = await injectContinuation({
        ctx: ctx as any,
        sessionID: "sess-1",
        message: "Hello",
        state
      })

      expect(result).toBe(true)
      expect(promptAsync).toHaveBeenCalledWith(expect.objectContaining({
        path: { id: "sess-1" },
        body: { parts: [{ type: "text", text: "Hello" }] }
      }))
      expect(state.lastInjectedAt).toBeGreaterThan(0)
      expect(state.awaitingPostInjectionProgressCheck).toBe(true)
    })

    it("should return false and record failure when promptAsync is unavailable", async () => {
      const state = mockSessionState()
      const ctx: OpenCodePromptContext = {
        directory: "/test",
      }

      const result = await injectContinuation({
        ctx,
        sessionID: "sess-1",
        message: "Hello",
        state,
      })

      expect(result).toBe(false)
      expect(state.lastInjectedAt).toBeGreaterThan(0)
      expect(state.consecutiveFailures).toBe(1)
    })

    it("should reset inFlight and record failure when promptAsync throws", async () => {
      const state = mockSessionState()
      const promptAsync = vi.fn().mockRejectedValue(new Error("boom"))
      const ctx: OpenCodePromptContext = {
        directory: "/test",
        client: {
          session: { promptAsync },
        },
      }

      const result = await injectContinuation({
        ctx,
        sessionID: "sess-1",
        message: "Hello",
        state,
      })

      expect(result).toBe(false)
      expect(promptAsync).toHaveBeenCalledOnce()
      expect(state.inFlight).toBe(false)
      expect(state.lastInjectedAt).toBeGreaterThan(0)
      expect(state.consecutiveFailures).toBe(1)
    })

    it("should refuse injection while already in flight", async () => {
      const state = mockSessionState()
      state.inFlight = true
      const promptAsync = vi.fn().mockResolvedValue({})
      const ctx: OpenCodePromptContext = {
        directory: "/test",
        client: {
          session: { promptAsync },
        },
      }

      await expect(injectContinuation({ ctx, sessionID: "sess-1", message: "Hello", state })).resolves.toBe(false)
      expect(promptAsync).not.toHaveBeenCalled()
    })
  })

  describe("injectAppendPrompt", () => {
    it("should append prompt through TUI when available", async () => {
      const appendPrompt = vi.fn().mockResolvedValue({})
      const ctx: OpenCodePromptContext = {
        directory: "/test",
        client: {
          tui: { appendPrompt },
        },
      }

      const result = await injectAppendPrompt({ ctx, message: "Append me" })

      expect(result).toBe(true)
      expect(appendPrompt).toHaveBeenCalledWith({
        body: { parts: [{ type: "text", text: "Append me" }] },
      })
    })

    it("should return false when TUI appendPrompt is unavailable", async () => {
      const ctx: OpenCodePromptContext = {
        directory: "/test",
      }

      await expect(injectAppendPrompt({ ctx, message: "Append me" })).resolves.toBe(false)
    })
  })
})
