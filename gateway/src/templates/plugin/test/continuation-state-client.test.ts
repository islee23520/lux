import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import * as fs from 'node:fs'
import * as path from 'node:path'
import {
  readContinuationState,
  writeContinuationState,
  updateContinuationState,
  type ContinuationState,
  type ContinuationStateWriteOptions,
} from '../continuation-state-client'

vi.mock('node:fs')

const mockFetch = vi.fn()
vi.stubGlobal('fetch', mockFetch)

describe('continuation-state-client', () => {
  const projectPath = '/test/project'
  const stateFilePath = path.join(projectPath, '.lux', 'continuation-state.json')
  const opts: ContinuationStateWriteOptions = {
    gatewayUrl: 'http://localhost:18766',
    projectPath,
    expectedSeq: 0,
  }

  beforeEach(() => {
    vi.resetAllMocks()
    vi.useFakeTimers()
    vi.setSystemTime(new Date('2026-05-13T12:00:00Z'))
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('readContinuationState', () => {
    it('returns default state when file does not exist', async () => {
      const state = await readContinuationState({ gatewayUrl: 'http://localhost:18766', projectPath })
      expect(state.status).toBe('Idle')
      expect(state.continuation_count).toBe(0)
    })

    it('returns parsed state when file exists', async () => {
      const mockState = {
        session_id: 'test-session',
        continuation_count: 5,
        stagnation_count: 1,
        consecutive_failures: 0,
        last_ambiguity: '0.5',
        last_ticket_baseline: null,
        current_ticket_id: 'TKT-1',
        status: 'Active',
        started_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
        stop_reason: null,
      }
      
      mockFetch.mockResolvedValue({
        ok: true,
        json: async () => mockState,
      })

      const state = await readContinuationState({ gatewayUrl: 'http://localhost:18766', projectPath })
      expect(state.session_id).toBe('test-session')
      expect(state.continuation_count).toBe(5)
      expect(state.status).toBe('Active')
    })

    it('returns default state when file is corrupt', async () => {
      mockFetch.mockResolvedValue({
        ok: false,
      })

      const state = await readContinuationState({ gatewayUrl: 'http://localhost:18766', projectPath })
      expect(state.status).toBe('Idle')
      expect(state.continuation_count).toBe(0)
    })
  })



  describe('writeContinuationState', () => {
    it('calls PUT /api/lux/continuation/state with correct payload', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ seq: 1 }),
        text: async () => '',
      })

      const state: ContinuationState = {
        session_id: 'test-session',
        continuation_count: 1,
        stagnation_count: 0,
        consecutive_failures: 0,
        last_ambiguity: null,
        last_ticket_baseline: null,
        current_ticket_id: null,
        status: 'Active',
        started_at: '2026-05-13T10:00:00Z',
        updated_at: '2026-05-13T10:00:00Z',
        stop_reason: null,
      }

      const result = await writeContinuationState(opts, state)

      expect(mockFetch).toHaveBeenCalledOnce()
      const [url, init] = mockFetch.mock.calls[0]
      expect(url).toContain('/api/lux/continuation/state')
      expect(init.method).toBe('PUT')
      const body = JSON.parse(init.body)
      expect(body.expected_seq).toBe(0)
      expect(body.status).toBe('Active')
      expect(result).toEqual({ seq: 1 })
    })

    it('throws on non-ok response', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      })
      const state = await readContinuationState({ gatewayUrl: 'http://localhost:18766', projectPath })

      mockFetch.mockResolvedValueOnce({
        ok: false,
        status: 409,
        text: async () => 'seq conflict',
      })

      await expect(writeContinuationState(opts, state)).rejects.toThrow('HTTP 409')
    })

    it('returns fallback seq when response body has no seq field', async () => {
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
      })
      const state = await readContinuationState({ gatewayUrl: 'http://localhost:18766', projectPath })

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({}),
        text: async () => '',
      })

      const result = await writeContinuationState({ ...opts, expectedSeq: 3 }, state)
      expect(result.seq).toBe(4)
    })
  })

  describe('updateContinuationState', () => {
    it('merges partial state and writes via API', async () => {
      const initialState: ContinuationState = {
        session_id: 'test-session',
        continuation_count: 1,
        stagnation_count: 0,
        consecutive_failures: 0,
        last_ambiguity: null,
        last_ticket_baseline: null,
        current_ticket_id: null,
        status: 'Active',
        started_at: '2026-05-13T10:00:00Z',
        updated_at: '2026-05-13T10:00:00Z',
        stop_reason: null,
      }

      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => initialState,
      })
      mockFetch.mockResolvedValueOnce({
        ok: true,
        json: async () => ({ seq: 2 }),
        text: async () => '',
      })

      const updated = await updateContinuationState(opts, {
        continuation_count: 2,
        status: 'Complete',
      })

      expect(updated.continuation_count).toBe(2)
      expect(updated.status).toBe('Complete')
      expect(updated.session_id).toBe('test-session')
    })
  })
})
