import { describe, it, expect } from 'vitest'
import { 
  formatStatus, 
  formatContextBlock, 
  buildToastMessage,
  type StatusInput,
  type SummaryInput,
  type DecisionInput
} from '../lux-overlay'

describe('lux-overlay', () => {
  describe('internal formatting branches', () => {
    it('renders health icons for all score bands', () => {
      expect(formatStatus({ status: 'Active' }, undefined, { healthScore: 75 })).toContain('💚 75')
      expect(formatStatus({ status: 'Active' }, undefined, { healthScore: 50 })).toContain('🟡 50')
      expect(formatStatus({ status: 'Active' }, undefined, { healthScore: 20 })).toContain('🔴 20')
    })

    it('renders undefined health scores as fallback dash', () => {
      expect(formatStatus({ status: 'Active' }, undefined, { healthScore: undefined })).not.toContain('💚')
    })

    it('renders unknown status icon fallback', () => {
      expect(formatStatus({ status: 'Paused' })).toContain('◔️ Paused')
    })

    it('shows ticket and failure defaults when fields are absent', () => {
      const result = formatStatus({ status: 'Active' }, { byStatus: {} }, { reason: 'ticket-work' })
      expect(result).toContain('⏸ ticket-work')
      expect(result).toContain('🎯 —')
    })

    it('truncates long status output', () => {
      const result = formatStatus({ status: 'A'.repeat(600) })
      expect(result.length).toBeLessThanOrEqual(500)
      expect(result.endsWith('…')).toBe(true)
    })

    it('shows dispatched next action and fallback idle target', () => {
      const dispatched = formatStatus(
        { status: 'Active' },
        undefined,
        { dispatched: true, reason: 'ticket-work', selectedTicketId: 'abc' },
      )
      expect(dispatched).toContain('→ abc')

      const idleFallback = formatStatus(
        { status: 'Active' },
        undefined,
        { dispatched: true, reason: 'ticket-work' },
      )
      expect(idleFallback).toContain('→ idle')
    })
  })

  describe('formatStatus', () => {
    it('formats basic status with minimal input', () => {
      const state: StatusInput = { status: 'Active', continuationCount: 5 }
      const result = formatStatus(state)
      
      expect(result).toContain('┌─ Lux Autonomous ─────────────────────┐')
      expect(result).toContain('✅ Active')
      expect(result).toContain('5')
      expect(result).toContain('└───────────────────────────────────┘')
    })

    it('formats status with summary and decision data', () => {
      const state: StatusInput = { 
        status: 'Active', 
        continuationCount: 3,
        currentTicketId: 't-001'
      }
      const summary: SummaryInput = {
        byStatus: { Done: 2 },
        totalTickets: 5,
        activeTicketsCount: 1
      }
      const decision: DecisionInput = {
        healthScore: 85,
        ambiguityScore: 10,
        dispatched: true,
        selectedTicketId: 't-002',
        reason: 'Next step'
      }

      const result = formatStatus(state, summary, decision)
      
      expect(result).toContain('✅ Active')
      expect(result).toContain('📋 2/5 done')
      expect(result).toContain('⚡1 active')
      expect(result).toContain('💚 85')
      expect(result).toContain('🔋 amb:10')
      expect(result).toContain('→ t-002')
      expect(result).toContain('🎯 t-001')
    })

    it('shows stop reason for terminal states', () => {
      const state: StatusInput = { 
        status: 'Complete', 
        stopReason: 'All goals met' 
      }
      const result = formatStatus(state)
      
      expect(result).toContain('🏁 Complete (All goals met)')
    })

    it('shows error icon and failures', () => {
      const state: StatusInput = { 
        status: 'Error', 
        consecutiveFailures: 3 
      }
      const result = formatStatus(state)
      
      expect(result).toContain('💥 Error')
      expect(result).toContain('⚠️3')
    })

    it('handles missing or undefined inputs gracefully', () => {
      const result = formatStatus(undefined)
      expect(result).toContain('◦️ Unknown')
      expect(result).toContain('?')
    })

    it('truncates long status messages', () => {
      const state: StatusInput = { 
        status: 'A'.repeat(600) 
      }
      const result = formatStatus(state)
      expect(result.length).toBeLessThanOrEqual(500)
      expect(result.endsWith('…')).toBe(true)
    })

    it('shows a waiting next action when not dispatched', () => {
      const state: StatusInput = { status: 'Active' }
      const decision: DecisionInput = { reason: 'ticket-work' }
      const result = formatStatus(state, undefined, decision)

      expect(result).toContain('⏸ ticket-work')
    })
  })

  describe('formatContextBlock', () => {
    it('returns fallback for missing summary', () => {
      expect(formatContextBlock(undefined)).toBe('[Lux: no data]')
    })

    it('formats summary data correctly', () => {
      const summary: SummaryInput = {
        byStatus: { Done: 5, ToDo: 3 },
        activeTicketsCount: 2,
        totalTickets: 10,
        incompleteCount: 5
      }
      const result = formatContextBlock(summary)
      
      expect(result).toContain('[Lux Status]')
      expect(result).toContain('Tickets: 5 Done / 3 ToDo / 2 InProgress (10 total)')
      expect(result).toContain('Remaining: 5')
    })

    it('handles alternative status casing (Todo vs ToDo)', () => {
      const summary: SummaryInput = {
        byStatus: { Done: 1, Todo: 2 },
        activeTicketsCount: 0,
        totalTickets: 3,
        incompleteCount: 2
      }
      const result = formatContextBlock(summary)
      expect(result).toContain('2 ToDo')
    })

    it('calculates total and incomplete if not provided', () => {
      const summary: SummaryInput = {
        byStatus: { Done: 2, ToDo: 3 },
        activeTicketsCount: 1
      }
      const result = formatContextBlock(summary)
      expect(result).toContain('2 Done')
      expect(result).toContain('3 ToDo')
    })
  })

  describe('buildToastMessage', () => {
    it('maps levels to variants correctly', () => {
      expect(buildToastMessage('info', ['msg']).variant).toBe('success')
      expect(buildToastMessage('warn', ['msg']).variant).toBe('error')
      expect(buildToastMessage('error', ['msg']).variant).toBe('error')
      expect(buildToastMessage('silent', ['msg']).variant).toBe('info')
    })

    it('joins sections with newlines', () => {
      const result = buildToastMessage('info', ['Line 1', 'Line 2'])
      expect(result.message).toBe('Line 1\nLine 2')
    })

    it('filters empty sections', () => {
      const result = buildToastMessage('info', ['Line 1', '', 'Line 2'])
      expect(result.message).toBe('Line 1\nLine 2')
    })

    it('truncates long messages', () => {
      const longStr = 'X'.repeat(600)
      const result = buildToastMessage('info', [longStr])
      expect(result.message.length).toBe(500)
      expect(result.message.endsWith('…')).toBe(true)
    })
  })
})
