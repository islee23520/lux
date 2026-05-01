import { useCallback } from 'react'
import type { NodeTypeDefinition, PipelineGraph, SavedGraphSummary } from '../types'

const TOKEN = 'dev-token'

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const response = await fetch(path, {
    ...init,
    headers: {
      'content-type': 'application/json',
      'x-lux-token': TOKEN,
      ...init.headers,
    },
  })

  if (!response.ok) {
    const body = await response.text()
    throw new Error(`${init.method ?? 'GET'} ${path} failed: ${response.status} ${body}`)
  }

  if (response.status === 204) {
    return undefined as T
  }

  return (await response.json()) as T
}

export function useGraphApi() {
  const fetchNodeTypes = useCallback(() => request<NodeTypeDefinition[]>('/api/node-types'), [])

  const saveGraph = useCallback((graph: PipelineGraph) => {
    return request<PipelineGraph>('/api/graphs', {
      method: 'POST',
      body: JSON.stringify(graph),
    })
  }, [])

  const loadGraphs = useCallback(() => request<SavedGraphSummary[]>('/api/graphs'), [])
  const loadGraph = useCallback((id: string) => request<PipelineGraph>(`/api/graphs/${encodeURIComponent(id)}`), [])

  const executeGraph = useCallback((id: string) => {
    return request<{ status: string; id: string }>(`/api/graphs/${encodeURIComponent(id)}/execute`, {
      method: 'POST',
    })
  }, [])

  const deleteGraph = useCallback((id: string) => {
    return request<void>(`/api/graphs/${encodeURIComponent(id)}`, { method: 'DELETE' })
  }, [])

  return { fetchNodeTypes, saveGraph, loadGraphs, loadGraph, executeGraph, deleteGraph }
}
