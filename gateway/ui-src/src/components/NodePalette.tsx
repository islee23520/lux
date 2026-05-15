import { useEffect, useMemo, useState } from 'react'
import type { NodeTypeDefinition } from '../types'

export const fallbackNodeTypes: NodeTypeDefinition[] = [
  {
    type: 'UnityContext',
    displayName: 'Unity Context',
    description: 'Reads scene, selection, and editor state.',
    category: 'context',
    inputPorts: [],
    outputPorts: [{ name: 'context', direction: 'Output', dataType: 'UnityContext' }],
    parameters: [{ name: 'scope', type: 'string', description: 'Context scope' }],
  },
  {
    type: 'OutputDirectory',
    displayName: 'Output Directory',
    description: 'Chooses where generated assets are written.',
    category: 'context',
    inputPorts: [],
    outputPorts: [{ name: 'directory', direction: 'Output', dataType: 'DirectoryPath' }],
    parameters: [{ name: 'path', type: 'string', description: 'Output path' }],
  },
  {
    type: 'PromptTemplate',
    displayName: 'Prompt Template',
    description: 'Combines context and reusable Codex Image prompt text.',
    category: 'pipeline',
    inputPorts: [
      { name: 'context', direction: 'Input', dataType: 'UnityContext' },
      { name: 'output', direction: 'Input', dataType: 'DirectoryPath' },
    ],
    outputPorts: [{ name: 'prompt', direction: 'Output', dataType: 'Prompt' }],
    parameters: [{ name: 'template', type: 'string', description: 'Prompt template' }],
  },
  {
    type: 'CodexGeneration',
    displayName: 'Codex Generation',
    description: 'Queues image generation through Lux tooling.',
    category: 'pipeline',
    inputPorts: [{ name: 'prompt', direction: 'Input', dataType: 'Prompt' }],
    outputPorts: [{ name: 'image', direction: 'Output', dataType: 'Image' }],
    parameters: [{ name: 'model', type: 'string', description: 'Generation model' }],
  },
  {
    type: 'Segmentation',
    displayName: 'Segmentation',
    description: 'Separates subject, mask, and background layers.',
    category: 'post-process',
    inputPorts: [{ name: 'image', direction: 'Input', dataType: 'Image' }],
    outputPorts: [{ name: 'mask', direction: 'Output', dataType: 'Mask' }],
    parameters: [{ name: 'threshold', type: 'number', description: 'Mask threshold' }],
  },
  {
    type: 'MaskPostProcessing',
    displayName: 'Mask Post Processing',
    description: 'Cleans masks and prepares Unity-ready exports.',
    category: 'post-process',
    inputPorts: [{ name: 'mask', direction: 'Input', dataType: 'Mask' }],
    outputPorts: [{ name: 'asset', direction: 'Output', dataType: 'UnityAsset' }],
    parameters: [{ name: 'format', type: 'string', description: 'Export format' }],
  },
]

export function NodePalette({
  fetchNodeTypes,
  onNodeTypes,
  latestEventLabel,
}: {
  fetchNodeTypes: () => Promise<NodeTypeDefinition[]>
  onNodeTypes: (types: NodeTypeDefinition[]) => void
  latestEventLabel: string
}) {
  const [query, setQuery] = useState('')
  const [nodeTypes, setNodeTypes] = useState<NodeTypeDefinition[]>(fallbackNodeTypes)
  const [source, setSource] = useState('fallback')

  useEffect(() => {
    let disposed = false
    fetchNodeTypes()
      .then((types) => {
        if (disposed) return
        const nextTypes = types.length > 0 ? types : fallbackNodeTypes
        setNodeTypes(nextTypes)
        setSource(types.length > 0 ? 'api' : 'fallback')
        onNodeTypes(nextTypes)
      })
      .catch((error: unknown) => {
        if (disposed) return
        console.warn('Using fallback node types', error)
        setNodeTypes(fallbackNodeTypes)
        setSource('fallback')
        onNodeTypes(fallbackNodeTypes)
      })
    return () => {
      disposed = true
    }
  }, [fetchNodeTypes, onNodeTypes])

  const grouped = useMemo(() => {
    const lowered = query.trim().toLowerCase()
    return nodeTypes
      .filter((type) => {
        if (!lowered) return true
        return `${type.displayName} ${type.description} ${type.type}`.toLowerCase().includes(lowered)
      })
      .reduce<Record<string, NodeTypeDefinition[]>>((groups, nodeType) => {
        const category = nodeType.category || 'uncategorized'
        return { ...groups, [category]: [...(groups[category] ?? []), nodeType] }
      }, {})
  }, [nodeTypes, query])

  return (
    <aside className="node-palette">
      <p className="eyebrow">Codex Image graph</p>
      <h2>Node Palette</h2>
      <p className="palette-hint">Drag a block into the canvas. Port data types must match.</p>
      <input
        className="palette-search"
        value={query}
        onChange={(event) => setQuery(event.target.value)}
        placeholder="Search nodes..."
      />
      <div className="event-preview">
        <span>Latest gateway event · {source}</span>
        <code>{latestEventLabel}</code>
      </div>
      <div className="palette-groups">
        {Object.entries(grouped).map(([category, types]) => (
          <section key={category} className="palette-group">
            <h3>{category}</h3>
            {types.map((type) => (
              <button
                key={type.type}
                className="palette-item"
                draggable
                onDragStart={(event) => {
                  event.dataTransfer.setData('application/lux-node-type', type.type)
                  event.dataTransfer.effectAllowed = 'copy'
                }}
              >
                <strong>{type.displayName}</strong>
                <span>{type.description}</span>
              </button>
            ))}
          </section>
        ))}
      </div>
    </aside>
  )
}
