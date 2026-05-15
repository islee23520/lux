import { useCallback } from 'react'
import type { AiLogEntry, AiLogContextEntry } from '../types'

export function useAiLogApi() {
  const fetchRecent = useCallback(async (limit = 50, filters?: {
    actor?: string
    category?: string
    source?: string
    action?: string
    eventType?: string
  }): Promise<AiLogEntry[]> => {
    const params = new URLSearchParams()
    params.set('limit', String(limit))
    if (filters?.actor) params.set('actor', filters.actor)
    if (filters?.category) params.set('category', filters.category)
    if (filters?.source) params.set('source', filters.source)
    if (filters?.action) params.set('action', filters.action)
    if (filters?.eventType) params.set('event_type', filters.eventType)
    const res = await fetch(`/api/ai-log?${params}`)
    if (!res.ok) throw new Error(`Failed to fetch AI log: ${res.status}`)
    return res.json()
  }, [])

  const fetchContext = useCallback(async (limit = 20): Promise<AiLogContextEntry[]> => {
    const params = new URLSearchParams()
    params.set('limit', String(limit))
    const res = await fetch(`/api/ai-log/context?${params}`)
    if (!res.ok) throw new Error(`Failed to fetch AI log context: ${res.status}`)
    return res.json()
  }, [])

  return { fetchRecent, fetchContext }
}
