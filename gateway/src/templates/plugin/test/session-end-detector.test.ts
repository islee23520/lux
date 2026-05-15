import { describe, it, expect, vi, beforeEach } from 'vitest'
import { setupSessionEndDetection, shouldAutoResume, formatSessionEndSummary, manualResume } from '../session-end-detector'
import type { SessionEndContext } from '../session-end-detector'

type TicketSummary = {
  tickets: Array<{ id: string; title?: string; status: string }>
  incompleteCount: number
}

const readContinuationStateMock = vi.hoisted(() => vi.fn(async () => ({ stop_reason: null })))
const loadTicketsMock = vi.hoisted(() => vi.fn((): TicketSummary => ({ tickets: [], incompleteCount: 0 })))
const getStateMock = vi.hoisted(() => vi.fn(() => ({ abortDetectedAt: undefined })))

vi.mock('../continuation-state-client', () => ({
  readContinuationState: readContinuationStateMock,
}))
vi.mock('../ticket-loader', () => ({
  loadTickets: loadTicketsMock,
}))
vi.mock('../session-state', () => ({
  getState: getStateMock,
}))
vi.mock('./continuation-state-client', () => ({
  readContinuationState: readContinuationStateMock,
}))
vi.mock('./ticket-loader', () => ({
  loadTickets: loadTicketsMock,
}))
vi.mock('./session-state', () => ({
  getState: getStateMock,
}))

describe('session-end-detector', () => {
  const projectPath = '/test/project'
  const onSessionEnd = vi.fn()

  function makeContext(overrides: Partial<SessionEndContext> = {}): SessionEndContext {
    return {
      reason: 'session.end',
      activeTickets: [],
      totalTickets: 0,
      incompleteCount: 0,
      projectPath,
      lastState: { stop_reason: null } as any,
      detectedAt: 100,
      ...overrides,
    }
  }

  beforeEach(() => {
    vi.restoreAllMocks()
    onSessionEnd.mockClear()
    loadTicketsMock.mockReturnValue({ tickets: [], incompleteCount: 0 })
    readContinuationStateMock.mockResolvedValue({ stop_reason: null })
    getStateMock.mockReturnValue({ abortDetectedAt: undefined })
  })


  describe('shouldAutoResume', () => {
    it('should return true if there are incomplete tickets and no abort', () => {
      expect(shouldAutoResume(1, null, undefined)).toBe(true)
    })

    it('should return false if no incomplete tickets', () => {
      expect(shouldAutoResume(0, null, undefined)).toBe(false)
    })

    it('should return false if user aborted', () => {
      expect(shouldAutoResume(1, 'user_abort', undefined)).toBe(false)
      expect(shouldAutoResume(1, null, Date.now())).toBe(false)
    })
  })

  describe('setupSessionEndDetection', () => {
    it('should register event handler and trigger onSessionEnd', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => {
        registeredHandler = handler
      }

      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1
      })

      setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)

      expect(registeredHandler).toBeDefined()

      await registeredHandler({ type: 'session.end', status: 'end' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({
        reason: 'session.end',
        incompleteCount: 1
      }))
    })

    it('should queue resume if processing is active', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => {
        registeredHandler = handler
      }

      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1
      })

      const detector = setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)
      detector.setProcessing(true)

      await registeredHandler({ type: 'session.end', status: 'end' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).not.toHaveBeenCalled()

      detector.setProcessing(false)
      await new Promise((resolve) => setTimeout(resolve, 0))
      expect(onSessionEnd).toHaveBeenCalled()
    })

    it('should handle manual resume trigger', async () => {
      const registerEvent = vi.fn()
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1
      })

      const detector = setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)
      detector.triggerManualResume()
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({
        reason: 'manual'
      }))
    })

    it('should support a plain registrar function and use the default state for empty project paths', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => {
        registeredHandler = handler
      }
      const debug = vi.spyOn(console, 'debug').mockImplementation(() => undefined)

      setupSessionEndDetection(registerEvent, onSessionEnd)

      await registeredHandler({ type: 'session.end' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(readContinuationStateMock).not.toHaveBeenCalled()
      expect(loadTicketsMock).not.toHaveBeenCalled()
      expect(debug).toHaveBeenCalledWith(expect.stringContaining('Incomplete: 0'))
      expect(onSessionEnd).not.toHaveBeenCalled()
    })

    it('should resolve shutdown and error events from different payload shapes', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => { registeredHandler = handler }
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1,
      })
      const warn = vi.spyOn(console, 'debug').mockImplementation(() => undefined)

      setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)

      await registeredHandler({ type: 'session.shutdown', message: 'done' })
      await new Promise((resolve) => setTimeout(resolve, 0))
      await registeredHandler({ type: 'session.error', properties: { message: 'error happened' } })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({ reason: 'session.shutdown' }))
      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({ reason: 'session.error' }))
      expect(warn).toHaveBeenCalled()
    })

    it('should ignore events after destroy and call the active listener cleanup', async () => {
      let registeredHandler: any
      const cleanup = vi.fn()
      const registerEvent = (handler: any) => {
        registeredHandler = handler
        return cleanup
      }
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1
      })

      const detector = setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)
      detector.destroy()

      await registeredHandler({ type: 'session.end', status: 'end' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(cleanup).toHaveBeenCalledTimes(1)
      expect(onSessionEnd).not.toHaveBeenCalled()
    })

    it('should queue a manual resume while processing and dispatch it when consumed', async () => {
      const registerEvent = vi.fn()
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', title: 'Task 1', status: 'todo' }],
        incompleteCount: 1
      })

      const detector = setupSessionEndDetection({ registerEvent, projectPath, getProcessing: () => true }, onSessionEnd)
      detector.triggerManualResume()
      await new Promise((resolve) => setTimeout(resolve, 0))
      expect(onSessionEnd).not.toHaveBeenCalled()

      detector.setProcessing(false)
      detector.consumePendingResume()
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).not.toHaveBeenCalled()
    })

    it('should use updated project path for subsequent session end events', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => {
        registeredHandler = handler
      }
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', title: 'Task 1', status: 'todo' }],
        incompleteCount: 1
      })

      const detector = setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)
      detector.setProjectPath('/new/path')

      await registeredHandler({ type: 'session.end', status: 'end' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(loadTicketsMock).toHaveBeenCalledWith('/new/path')
      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({
        projectPath: '/new/path',
        reason: 'session.end'
      }))
    })
  })

  describe('manualResume', () => {
    it('should return a manual context and call the callback when auto-resume is allowed', () => {
      vi.spyOn(Date, 'now').mockReturnValue(222)
      const callback = vi.fn()
      const ctx = makeContext({
        incompleteCount: 2,
        activeTickets: [{ id: 'T1', title: 'Task 1', status: 'todo' }],
        totalTickets: 2,
        lastState: { stop_reason: null } as any,
      })

      const resumeCtx = manualResume(ctx, callback)

      expect(resumeCtx).toEqual(expect.objectContaining({
        reason: 'manual',
        detectedAt: 222,
        incompleteCount: 2,
      }))
      expect(callback).toHaveBeenCalledWith(resumeCtx)
      expect(ctx.reason).toBe('session.end')
    })
  })

  describe('formatSessionEndSummary', () => {
    it('should format summary string', () => {
      const ctx = makeContext({
        activeTickets: [{ id: 'T1', title: 'Task 1', status: 'todo' }],
        totalTickets: 1,
        incompleteCount: 1,
        projectPath: '/test',
        detectedAt: Date.now(),
      })
      const summary = formatSessionEndSummary(ctx)
      expect(summary).toContain('Session ended: session.end')
      expect(summary).toContain('Active tickets remaining: 1/1')
    })

    it('should include remaining ticket details when active tickets are present', () => {
      const summary = formatSessionEndSummary(makeContext({
        activeTickets: [
          { id: 'T1', title: 'First task', status: 'todo' },
          { id: 'T2', title: 'Second task', status: 'in_progress' },
          { id: 'T3', title: 'Third task', status: 'todo' },
        ],
        totalTickets: 4,
        incompleteCount: 3,
      }))

      expect(summary).toContain('Remaining:')
      expect(summary).toContain('  [todo] First task (T1)')
      expect(summary).toContain('  [in_progress] Second task (T2)')
      expect(summary).toContain('  [todo] Third task (T3)')
    })

    it('should auto-resume only when permitted', () => {
      const ctx = makeContext({ incompleteCount: 1, lastState: { stop_reason: 'user_abort' } as any })
      const callback = vi.fn()

      const resumeCtx = manualResume(ctx, callback)

      expect(resumeCtx.reason).toBe('manual')
      expect(callback).not.toHaveBeenCalled()
    })

    it('should return false for unknown event types', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => { registeredHandler = handler }
      setupSessionEndDetection({ registerEvent, projectPath }, onSessionEnd)

      await registeredHandler({ type: 'tool.execute' })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(onSessionEnd).not.toHaveBeenCalled()
    })

    it('should use properties.directory and message-based error detection', async () => {
      let registeredHandler: any
      const registerEvent = (handler: any) => { registeredHandler = handler }
      loadTicketsMock.mockReturnValue({
        tickets: [{ id: 'T1', status: 'todo' }],
        incompleteCount: 1,
      })

      setupSessionEndDetection({ registerEvent }, onSessionEnd)

      await registeredHandler({ type: 'session.update', properties: { directory: '/props/project', message: 'error' } })
      await new Promise((resolve) => setTimeout(resolve, 0))

      expect(loadTicketsMock).toHaveBeenCalledWith('/props/project')
      expect(onSessionEnd).toHaveBeenCalledWith(expect.objectContaining({ reason: 'session.error' }))
    })

  })
})
