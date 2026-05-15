import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import * as fs from 'node:fs'
import * as path from 'node:path'
import {
  loadTickets,
  invalidateCache,
  setCacheTtl,
  countByStatus,
  getActiveTickets,
  getTicketById,
  normalizeStatus,
  STATUS_MAP,
  type Ticket
} from '../ticket-loader'

vi.mock('node:fs')

describe('ticket-loader', () => {
  const projectPath = '/test/project'
  const ticketsDir = path.join(projectPath, '.lux', 'tickets')

  beforeEach(() => {
    vi.clearAllMocks()
    invalidateCache()
    setCacheTtl(10000)
    vi.useFakeTimers()
  })

  afterEach(() => {
    vi.useRealTimers()
  })

  describe('normalizeStatus', () => {
    it('should normalize common status strings', () => {
      expect(normalizeStatus('todo')).toBe('ToDo')
      expect(normalizeStatus('todo_')).toBe('ToDo')
      expect(normalizeStatus('inprogress')).toBe('InProgress')
      expect(normalizeStatus('in_progress')).toBe('InProgress')
      expect(normalizeStatus('backlog')).toBe('Backlog')
      expect(normalizeStatus('blocked')).toBe('Blocked')
      expect(normalizeStatus('done')).toBe('Done')
    })

    it('should handle whitespace and casing', () => {
      expect(normalizeStatus('  TODO  ')).toBe('ToDo')
      expect(normalizeStatus('In Progress')).toBe('InProgress')
    })

    it('should return original string if no match found', () => {
      expect(normalizeStatus('CustomStatus')).toBe('CustomStatus')
    })
  })

  describe('countByStatus', () => {
    it('should count tickets by status', () => {
      const tickets: Ticket[] = [
        { id: '1', status: 'ToDo' },
        { id: '2', status: 'ToDo' },
        { id: '3', status: 'InProgress' },
        { id: '4', status: 'Done' },
      ]
      const counts = countByStatus(tickets)
      expect(counts).toEqual({
        ToDo: 2,
        InProgress: 1,
        Done: 1
      })
    })

    it('should return empty object for empty tickets array', () => {
      expect(countByStatus([])).toEqual({})
    })
  })

  describe('getActiveTickets', () => {
    it('should filter tickets that are ToDo or InProgress and have spec_ref', () => {
      const tickets: Ticket[] = [
        { id: '1', status: 'ToDo', spec_ref: 'spec-1' },
        { id: '2', status: 'InProgress', spec_ref: 'spec-2' },
        { id: '3', status: 'ToDo' }, // missing spec_ref
        { id: '4', status: 'Done', spec_ref: 'spec-4' }, // wrong status
        { id: '5', status: 'Backlog', spec_ref: 'spec-5' }, // wrong status
      ]
      const active = getActiveTickets(tickets)
      expect(active).toHaveLength(2)
      expect(active.map(t => t.id)).toEqual(['1', '2'])
    })
  })

  describe('loadTickets', () => {
    it('should return empty summary if tickets directory does not exist', () => {
      vi.mocked(fs.readdirSync).mockImplementation(() => {
        throw new Error('ENOENT')
      })

      const summary = loadTickets(projectPath)
      expect(summary.tickets).toEqual([])
      expect(summary.incompleteCount).toBe(0)
      expect(summary.activeTickets).toEqual([])
    })

    it('should load and normalize tickets from JSON files', () => {
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json', 't2.json', 'not-json.txt'] as any)
      vi.mocked(fs.readFileSync).mockImplementation((p: any) => {
        if (p.endsWith('t1.json')) {
          return JSON.stringify({ id: 't1', status: 'todo', spec_ref: 's1', tags: ['tag1'] })
        }
        if (p.endsWith('t2.json')) {
          return JSON.stringify({ id: 't2', status: 'done' })
        }
        return ''
      })

      const summary = loadTickets(projectPath)
      expect(summary.tickets).toHaveLength(2)
      expect(summary.tickets[0]).toMatchObject({
        id: 't1',
        status: 'ToDo',
        spec_ref: 's1',
        tags: ['tag1']
      })
      expect(summary.tickets[1].status).toBe('Done')
      expect(summary.incompleteCount).toBe(1)
      expect(summary.byStatus).toEqual({ ToDo: 1, Done: 1 })
    })

    it('should skip malformed JSON files', () => {
      const consoleSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      vi.mocked(fs.readdirSync).mockReturnValue(['bad.json', 'good.json'] as any)
      vi.mocked(fs.readFileSync).mockImplementation((p: any) => {
        if (p.endsWith('bad.json')) return 'invalid json'
        return JSON.stringify({ id: 'good', status: 'done' })
      })

      const summary = loadTickets(projectPath)
      expect(summary.tickets).toHaveLength(1)
      expect(summary.tickets[0].id).toBe('good')
      expect(consoleSpy).toHaveBeenCalled()
      consoleSpy.mockRestore()
    })

    it('should use cache if within TTL', () => {
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json'] as any)
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ id: 't1', status: 'todo' }))

      const first = loadTickets(projectPath)
      expect(fs.readdirSync).toHaveBeenCalledTimes(1)

      const second = loadTickets(projectPath)
      expect(second).toBe(first)
      expect(fs.readdirSync).toHaveBeenCalledTimes(1)

      // Advance time past TTL
      vi.advanceTimersByTime(11000)
      loadTickets(projectPath)
      expect(fs.readdirSync).toHaveBeenCalledTimes(2)
    })

    it('should invalidate cache when invalidateCache is called', () => {
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json'] as any)
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ id: 't1', status: 'todo' }))

      loadTickets(projectPath)
      expect(fs.readdirSync).toHaveBeenCalledTimes(1)

      invalidateCache()
      loadTickets(projectPath)
      expect(fs.readdirSync).toHaveBeenCalledTimes(2)
    })

    it('should respect custom cache TTL', () => {
      setCacheTtl(500)
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json'] as any)
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ id: 't1', status: 'todo' }))

      loadTickets(projectPath)
      vi.advanceTimersByTime(600)
      loadTickets(projectPath)
      expect(fs.readdirSync).toHaveBeenCalledTimes(2)
    })
  })

  describe('getTicketById', () => {
    it('should return ticket by ID if found', () => {
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json'] as any)
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ id: 't1', status: 'todo' }))

      const ticket = getTicketById(projectPath, 't1')
      expect(ticket).not.toBeNull()
      expect(ticket?.id).toBe('t1')
    })

    it('should return null if ticket ID not found', () => {
      vi.mocked(fs.readdirSync).mockReturnValue(['t1.json'] as any)
      vi.mocked(fs.readFileSync).mockReturnValue(JSON.stringify({ id: 't1', status: 'todo' }))

      const ticket = getTicketById(projectPath, 'non-existent')
      expect(ticket).toBeNull()
    })
  })
})
