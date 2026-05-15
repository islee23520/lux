import { useEffect, useState } from 'react'
import { useToolApi } from '../hooks/useToolApi'
import type { AvailableTool, ToolSession } from '../types'

interface ToolSelectorProps {
  activeTool: string
  onSelectTool: (toolType: string) => void
  sessions: Map<string, ToolSession>
}

export function ToolSelector({ activeTool, onSelectTool, sessions }: ToolSelectorProps) {
  const { fetchAvailableTools } = useToolApi()
  const [tools, setTools] = useState<AvailableTool[]>([])
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    fetchAvailableTools()
      .then(setTools)
      .catch(console.error)
      .finally(() => setLoading(false))
  }, [fetchAvailableTools])

  if (loading) {
    return <div className="tool-selector loading">Loading tools...</div>
  }

  return (
    <div className="tool-selector">
      <h3 className="eyebrow">Active Tool</h3>
      <div className="tool-list">
        {tools.map((tool) => {
          const session = sessions.get(tool.type)
          const status = session?.status || 'disconnected'
          
          return (
            <button
              key={tool.type}
              className={`tool-tab ${activeTool === tool.type ? 'active' : ''}`}
              onClick={() => onSelectTool(tool.type)}
            >
              <span className="tool-name">{tool.displayName}</span>
              <span className={`status-dot status-${status}`} title={status} />
            </button>
          )
        })}
      </div>
    </div>
  )
}
