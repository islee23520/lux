import { useState } from 'react'
import type { PipelineGraph, SavedGraphSummary } from '../types'

export function Toolbar({
  canUndo,
  canRedo,
  onUndo,
  onRedo,
  toPipelineGraph,
  fromPipelineGraph,
  saveGraph,
  loadGraphs,
  loadGraph,
  executeGraph,
}: {
  canUndo: boolean
  canRedo: boolean
  onUndo: () => void
  onRedo: () => void
  toPipelineGraph: () => PipelineGraph
  fromPipelineGraph: (graph: PipelineGraph) => void
  saveGraph: (graph: PipelineGraph) => Promise<PipelineGraph>
  loadGraphs: () => Promise<SavedGraphSummary[]>
  loadGraph: (id: string) => Promise<PipelineGraph>
  executeGraph: (id: string) => Promise<{ status: string; id: string }>
}) {
  const [status, setStatus] = useState('Ready')
  const [busy, setBusy] = useState(false)
  const [currentGraphId, setCurrentGraphId] = useState('lux-graph')

  const run = async (label: string, action: () => Promise<void>) => {
    setBusy(true)
    setStatus(`${label}...`)
    try {
      await action()
    } catch (error) {
      setStatus(error instanceof Error ? error.message : `${label} failed`)
    } finally {
      setBusy(false)
    }
  }

  const handleSave = () =>
    run('Saving graph', async () => {
      const saved = await saveGraph(toPipelineGraph())
      setCurrentGraphId(saved.id)
      setStatus(`Saved ${saved.displayName || saved.id}`)
    })

  const handleLoad = () =>
    run('Loading graphs', async () => {
      const graphs = await loadGraphs()
      if (graphs.length === 0) {
        setStatus('No saved graphs')
        return
      }
      const options = graphs.map((graph, index) => `${index + 1}. ${graph.displayName || graph.id}`).join('\n')
      const choice = window.prompt(`Load graph:\n${options}`, '1')
      if (!choice) {
        setStatus('Load cancelled')
        return
      }
      const index = Number.parseInt(choice, 10) - 1
      const summary = graphs[index]
      if (!summary) {
        setStatus('Invalid graph selection')
        return
      }
      const graph = await loadGraph(summary.id)
      fromPipelineGraph(graph)
      setCurrentGraphId(graph.id)
      setStatus(`Loaded ${graph.displayName || graph.id}`)
    })

  const handleExecute = () =>
    run('Executing graph', async () => {
      const result = await executeGraph(currentGraphId)
      setStatus(`Execute ${result.status || 'queued'} · ${result.id || currentGraphId}`)
    })

  return (
    <div className="editor-toolbar">
      <div className="toolbar-actions">
        <button onClick={handleSave} disabled={busy}>Save</button>
        <button onClick={handleLoad} disabled={busy}>Load</button>
        <button className="execute-button" onClick={handleExecute} disabled={busy}>Execute Pipeline</button>
        <span className="toolbar-divider" />
        <button onClick={onUndo} disabled={!canUndo || busy}>Undo</button>
        <button onClick={onRedo} disabled={!canRedo || busy}>Redo</button>
      </div>
      <div className="toolbar-status">{status}</div>
    </div>
  )
}
