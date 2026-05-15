import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { createProgressPoller } from '../progress-poller'

const fetchMock = vi.fn()
globalThis.fetch = fetchMock

describe('progress-poller', () => {
  const config = {
    gatewayUrl: 'http://localhost:17340',
    projectPath: '/test/project',
    pollIntervalMs: 100,
    onProgress: vi.fn(),
    onError: vi.fn()
  }

  beforeEach(() => {
    vi.useFakeTimers()
    fetchMock.mockReset()
    config.onProgress.mockClear()
    config.onError.mockClear()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  it('should fetch progress and call onProgress on meaningful change', async () => {
    const summary1 = {
      kanban: {
        tickets: [{ id: 'T1', title: 'Task 1', status: 'Todo' }]
      }
    }
    fetchMock.mockResolvedValueOnce({
      ok: true,
      json: async () => summary1
    })

    const poller = createProgressPoller(config)
    poller.start()

    await vi.runOnlyPendingTimersAsync()

    expect(fetchMock).toHaveBeenCalledWith(expect.stringContaining('/api/lux/progress/summary'), expect.any(Object))
    expect(config.onProgress).toHaveBeenCalledWith(expect.objectContaining({
      currentSummary: summary1,
      previousSummary: null
    }))
  })

  it('should detect ticket status changes', async () => {
    const summary1 = {
      kanban: {
        tickets: [{ id: 'T1', title: 'Task 1', status: 'Todo' }]
      }
    }
    const summary2 = {
      kanban: {
        tickets: [{ id: 'T1', title: 'Task 1', status: 'Done' }]
      }
    }

    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary1 })
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary2 })

    const poller = createProgressPoller(config)
    poller.start()

    await vi.runOnlyPendingTimersAsync()
    expect(config.onProgress).toHaveBeenCalledTimes(2)

    await vi.advanceTimersByTimeAsync(100)
    await vi.runOnlyPendingTimersAsync()

    expect(config.onProgress).toHaveBeenCalledTimes(2)
    expect(config.onProgress).toHaveBeenLastCalledWith(expect.objectContaining({
      changedTickets: [{
        id: 'T1',
        title: 'Task 1',
        previousStatus: 'Todo',
        newStatus: 'Done'
      }]
    }))
  })

  it('should handle fetch errors and retry with backoff', async () => {
    fetchMock.mockRejectedValue(new Error('Network error'))

    const poller = createProgressPoller(config)
    poller.start()

    await vi.runOnlyPendingTimersAsync()

    expect(config.onError).toHaveBeenCalled()
    
    for (let i = 0; i < 3; i++) {
      await vi.runOnlyPendingTimersAsync()
    }
    
    expect(fetchMock).toHaveBeenCalledTimes(5)
  })

  it('should stop polling when stop() is called', async () => {
    fetchMock.mockResolvedValue({
      ok: true,
      json: async () => ({})
    })

    const poller = createProgressPoller(config)
    poller.start()
    poller.stop()

    await vi.advanceTimersByTimeAsync(1000)
    expect(fetchMock).toHaveBeenCalledTimes(1)
  })

  it('should resolve retry delay immediately when stopped from the error handler', async () => {
    const poller = createProgressPoller(config)
    config.onError.mockImplementationOnce(() => {
      poller.stop()
    })
    fetchMock.mockRejectedValue(new Error('Network error'))

    poller.start()
    await vi.runAllTimersAsync()

    expect(config.onError).toHaveBeenCalledTimes(1)
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('should resolve an in-flight retry delay when stopped mid-delay', async () => {
    fetchMock.mockRejectedValue(new Error('Network error'))

    const poller = createProgressPoller(config)
    poller.start()

    await vi.waitFor(() => {
      expect(config.onError).toHaveBeenCalledTimes(1)
    })

    poller.stop()
    await vi.runOnlyPendingTimersAsync()

    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('should compute deltas from activeTickets when no status counts are provided', async () => {
    const summary = {
      kanban: {
        activeTickets: [{ id: 'T1', title: 'Task 1', status: 'ToDo' }]
      }
    }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary })

    const poller = createProgressPoller(config)
    await poller.poll()

    expect(config.onProgress).toHaveBeenCalledWith(expect.objectContaining({
      currentSummary: summary,
      progressDelta: { ToDo: 1 }
    }))
  })

  it('should compute deltas from active_tickets fallback when no direct tickets exist', async () => {
    const summary = {
      kanban: {
        active_tickets: [{ id: 'T1', status: 'Done' }]
      }
    }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary })

    const poller = createProgressPoller(config)
    await poller.poll()

    expect(config.onProgress).toHaveBeenCalledWith(expect.objectContaining({
      currentSummary: summary,
      progressDelta: { Done: 1 }
    }))
  })

  it('should ignore non-finite status counts', async () => {
    const summary = {
      kanban: {
        byStatus: {
          Done: 5,
          InProgress: Infinity,
          Todo: NaN
        }
      }
    }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary })

    const poller = createProgressPoller(config)
    await poller.poll()

    expect(config.onProgress).toHaveBeenCalledWith(expect.objectContaining({
      progressDelta: { Done: 5 }
    }))
  })

  it('should route non-abort errors from poll through onError', async () => {
    const networkError = new Error('Network error')
    const errorHandlerFailure = new Error('Error handler failed')
    fetchMock.mockRejectedValueOnce(networkError)
    config.onError.mockImplementationOnce(() => {
      throw errorHandlerFailure
    })

    const poller = createProgressPoller(config)
    await poller.poll()

    expect(config.onError).toHaveBeenCalledWith(errorHandlerFailure)
  })

  it('should create a fresh abort controller when restarted after stop', async () => {
    fetchMock.mockImplementation(async (_url: string, init?: RequestInit) => {
      if (init?.signal?.aborted) throw new Error('Signal was already aborted')
      return { ok: true, json: async () => ({ kanban: { byStatus: { Done: 1 } } }) }
    })

    const poller = createProgressPoller(config)
    poller.start()
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(1)
    })
    poller.stop()

    poller.start()
    await vi.waitFor(() => {
      expect(fetchMock).toHaveBeenCalledTimes(2)
    })

    expect(config.onError).not.toHaveBeenCalledWith(expect.objectContaining({
      message: 'Signal was already aborted'
    }))
    expect(fetchMock).toHaveBeenCalledTimes(2)
  })

  it('should back off after repeated failures beyond the retry limit', async () => {
    fetchMock.mockRejectedValue(new Error('Network error'))

    const poller = createProgressPoller(config)
    const pollPromise = poller.poll()

    await vi.advanceTimersByTimeAsync(1000)
    await vi.advanceTimersByTimeAsync(2000)
    await vi.advanceTimersByTimeAsync(4000)
    await pollPromise

    expect(fetchMock).toHaveBeenCalledTimes(4)
    expect(config.onError).toHaveBeenCalled()
  })

  it('should ignore non-meaningful progress diffs', async () => {
    const summary = { kanban: { tickets: [] } }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary })

    const poller = createProgressPoller(config)
    await poller.poll()

    expect(config.onProgress).toHaveBeenCalledWith(expect.objectContaining({
      previousSummary: null,
      currentSummary: summary,
    }))
  })

  it('should handle by_status counts and aborted polls', async () => {
    const summary = {
      kanban: {
        by_status: { Done: 2, InProgress: 1 },
      }
    }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => summary })

    const poller = createProgressPoller(config)
    poller.stop()
    await poller.poll()

    expect(config.onProgress).not.toHaveBeenCalled()
  })

  it('should compute deltas from byStatus counts on a second poll', async () => {
    const first = { kanban: { byStatus: { Done: 1 } } }
    const second = { kanban: { byStatus: { Done: 2, Todo: 1 } } }
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => first })
    fetchMock.mockResolvedValueOnce({ ok: true, json: async () => second })

    const poller = createProgressPoller(config)
    await poller.poll()
    await poller.poll()

    expect(config.onProgress).toHaveBeenLastCalledWith(expect.objectContaining({
      progressDelta: { Done: 1, Todo: 1 },
    }))
  })
})
