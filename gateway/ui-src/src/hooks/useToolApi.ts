import { useCallback } from 'react'
import type { AvailableTool, ToolSession, ToolExecution } from '../types'

export function useToolApi() {
  const fetchAvailableTools = useCallback(async (): Promise<AvailableTool[]> => {
    const res = await fetch('/api/tools')
    if (!res.ok) throw new Error(`Failed to fetch tools: ${res.status} ${res.statusText}`)
    return res.json()
  }, [])

  const createSession = useCallback(async (toolType: string): Promise<ToolSession> => {
    const res = await fetch('/api/tools/sessions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ toolType })
    })
    if (!res.ok) throw new Error(`Failed to create session: ${res.status} ${res.statusText}`)
    return res.json()
  }, [])

  const executeCommand = useCallback(async (toolType: string, command: string, sessionId?: string): Promise<ToolExecution> => {
    const res = await fetch('/api/tools/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ toolType, command, sessionId })
    })
    if (!res.ok) throw new Error(`Failed to execute command: ${res.status} ${res.statusText}`)
    return res.json()
  }, [])

  const executeSkill = useCallback(async (toolType: string, skillName: string, params?: Record<string, unknown>): Promise<ToolExecution> => {
    const res = await fetch('/api/tools/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ toolType, skillName, params })
    })
    if (!res.ok) throw new Error(`Failed to execute skill: ${res.status} ${res.statusText}`)
    return res.json()
  }, [])

  const getSession = useCallback(async (sessionId: string): Promise<ToolSession> => {
    const res = await fetch(`/api/tools/sessions/${sessionId}`)
    if (!res.ok) throw new Error('Failed to get session')
    return res.json()
  }, [])

  const getExecution = useCallback(async (executionId: string): Promise<ToolExecution> => {
    const res = await fetch(`/api/tools/executions/${executionId}`)
    if (!res.ok) throw new Error('Failed to get execution')
    return res.json()
  }, [])

  const deleteSession = useCallback(async (sessionId: string): Promise<void> => {
    const res = await fetch(`/api/tools/sessions/${sessionId}`, {
      method: 'DELETE'
    })
    if (!res.ok) throw new Error('Failed to delete session')
  }, [])

  return {
    fetchAvailableTools,
    createSession,
    executeCommand,
    executeSkill,
    getSession,
    getExecution,
    deleteSession
  }
}
