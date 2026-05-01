import { useCallback } from 'react'
import type { AvailableTool, ToolSession, ToolExecution } from '../types'

export function useToolApi() {
  const fetchAvailableTools = useCallback(async (): Promise<AvailableTool[]> => {
    try {
      const res = await fetch('/api/tools')
      if (!res.ok) throw new Error('Failed to fetch tools')
      return res.json()
    } catch (e) {
      console.warn('Failed to fetch tools, using mock data', e)
      return [
        { type: 'claude-code', displayName: 'Claude Code', description: 'Anthropic Claude', integrationMethod: 'api', capabilities: [], status: 'ready' },
        { type: 'openai-codex', displayName: 'OpenAI Codex', description: 'OpenAI Codex', integrationMethod: 'api', capabilities: [], status: 'ready' },
        { type: 'opencode', displayName: 'OpenCode', description: 'OpenCode', integrationMethod: 'api', capabilities: [], status: 'ready' }
      ]
    }
  }, [])

  const createSession = useCallback(async (toolType: string): Promise<ToolSession> => {
    try {
      const res = await fetch('/api/tools/sessions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ toolType })
      })
      if (!res.ok) throw new Error('Failed to create session')
      return res.json()
    } catch (e) {
      console.warn('Failed to create session, using mock data', e)
      return {
        id: crypto.randomUUID(),
        toolType,
        status: 'connected',
        createdAtUtc: new Date().toISOString(),
        updatedAtUtc: new Date().toISOString(),
        commandHistory: [],
        lastOutput: null
      }
    }
  }, [])

  const executeCommand = useCallback(async (toolType: string, command: string, sessionId?: string): Promise<ToolExecution> => {
    try {
      const res = await fetch('/api/tools/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ toolType, command, sessionId })
      })
      if (!res.ok) throw new Error('Failed to execute command')
      return res.json()
    } catch (e) {
      console.warn('Failed to execute command, using mock data', e)
      return {
        id: crypto.randomUUID(),
        toolSessionId: sessionId || crypto.randomUUID(),
        command,
        status: 'completed',
        createdAtUtc: new Date().toISOString(),
        updatedAtUtc: new Date().toISOString(),
        output: `Executed ${command} on ${toolType}`,
        error: null
      }
    }
  }, [])

  const executeSkill = useCallback(async (toolType: string, skillName: string, params?: Record<string, unknown>): Promise<ToolExecution> => {
    try {
      const res = await fetch('/api/tools/execute', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ toolType, skillName, params })
      })
      if (!res.ok) throw new Error('Failed to execute skill')
      return res.json()
    } catch (e) {
      console.warn('Failed to execute skill, using mock data', e)
      return {
        id: crypto.randomUUID(),
        toolSessionId: crypto.randomUUID(),
        command: `skill ${skillName}`,
        status: 'completed',
        createdAtUtc: new Date().toISOString(),
        updatedAtUtc: new Date().toISOString(),
        output: `Executed skill ${skillName} on ${toolType}`,
        error: null
      }
    }
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
