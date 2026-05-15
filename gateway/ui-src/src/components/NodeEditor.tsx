import { useCallback, useMemo, useState } from 'react'
import ReactFlow, {
  Background,
  Controls,
  Handle,
  MiniMap,
  Position,
  ReactFlowProvider,
  useReactFlow,
  type NodeMouseHandler,
  type NodeProps,
  type NodeTypes,
} from 'reactflow'
import type { GraphEditor, LuxEventEnvelope, LuxNodeData, NodeTypeDefinition } from '../types'
import { useGraphApi } from '../hooks/useGraphApi'
import { useGraphEditor } from '../hooks/useGraphEditor'
import { NodePalette } from './NodePalette'
import { PropertyPanel } from './PropertyPanel'
import { Toolbar } from './Toolbar'

function LuxPipelineNode({ data, selected }: NodeProps<LuxNodeData>) {
  return (
    <div className={`lux-node lux-node--${data.status} ${selected ? 'lux-node--selected' : ''}`}>
      <div className="lux-node__ports lux-node__ports--input">
        {data.inputPorts.map((port, index) => (
          <Handle
            key={port.name}
            id={port.name}
            type="target"
            position={Position.Left}
            className="lux-handle lux-handle--input"
            style={{ top: `${((index + 1) / (data.inputPorts.length + 1)) * 100}%` }}
          />
        ))}
      </div>
      <div className="lux-node__eyebrow">{data.pipelineType}</div>
      <div className="lux-node__title">{data.label}</div>
      <p>{data.description}</p>
      <div className="lux-node__badges">
        <span>{data.category}</span>
        <span>{data.status}</span>
      </div>
      <div className="lux-node__ports lux-node__ports--output">
        {data.outputPorts.map((port, index) => (
          <Handle
            key={port.name}
            id={port.name}
            type="source"
            position={Position.Right}
            className="lux-handle lux-handle--output"
            style={{ top: `${((index + 1) / (data.outputPorts.length + 1)) * 100}%` }}
          />
        ))}
      </div>
    </div>
  )
}

const nodeTypes: NodeTypes = {
  luxPipeline: LuxPipelineNode,
}

function NodeEditorCanvas({
  editor,
  nodeTypesByName,
}: {
  editor: GraphEditor
  nodeTypesByName: Map<string, NodeTypeDefinition>
}) {
  const reactFlow = useReactFlow<LuxNodeData>()

  const onDrop = useCallback(
    (event: React.DragEvent<HTMLDivElement>) => {
      event.preventDefault()
      const typeName = event.dataTransfer.getData('application/lux-node-type')
      const typeDef = nodeTypesByName.get(typeName)
      if (!typeDef) return
      const position = reactFlow.screenToFlowPosition({ x: event.clientX, y: event.clientY })
      editor.addNode(typeDef, position)
    },
    [editor, nodeTypesByName, reactFlow],
  )

  const onNodeClick: NodeMouseHandler = useCallback(
    (_event, node) => {
      editor.setSelectedNodeId(node.id)
      editor.setSelectedEdgeId(null)
    },
    [editor],
  )

  return (
    <div className="flow-frame" onDrop={onDrop} onDragOver={(event) => event.preventDefault()}>
      <ReactFlow
        nodes={editor.nodes}
        edges={editor.edges}
        nodeTypes={nodeTypes}
        onNodesChange={editor.onNodesChange}
        onEdgesChange={editor.onEdgesChange}
        onConnect={editor.addEdge}
        onNodeClick={onNodeClick}
        onEdgeClick={(_event, edge) => {
          editor.setSelectedEdgeId(edge.id)
          editor.setSelectedNodeId(null)
        }}
        onPaneClick={() => {
          editor.setSelectedNodeId(null)
          editor.setSelectedEdgeId(null)
        }}
        deleteKeyCode={['Backspace', 'Delete']}
        fitView
        proOptions={{ hideAttribution: true }}
      >
        <Background color="#364156" gap={26} size={1.2} />
        <MiniMap pannable zoomable nodeStrokeWidth={3} />
        <Controls />
      </ReactFlow>
    </div>
  )
}

function NodeEditorInner({ latestEvent }: { latestEvent?: LuxEventEnvelope }) {
  const editor = useGraphEditor()
  const api = useGraphApi()
  const [availableTypes, setAvailableTypes] = useState<NodeTypeDefinition[]>([])

  const latestEventLabel = latestEvent ? `${latestEvent.category}:${latestEvent.source}` : 'Waiting for /events'
  const nodeTypesByName = useMemo(
    () => new Map(availableTypes.map((nodeType) => [nodeType.type, nodeType])),
    [availableTypes],
  )

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      const meta = event.metaKey || event.ctrlKey
      if (!meta) return
      if (event.key.toLowerCase() === 'z' && event.shiftKey) {
        event.preventDefault()
        editor.redo()
      } else if (event.key.toLowerCase() === 'z') {
        event.preventDefault()
        editor.undo()
      } else if (event.key.toLowerCase() === 'c') {
        event.preventDefault()
        editor.copy()
      } else if (event.key.toLowerCase() === 'v') {
        event.preventDefault()
        editor.paste()
      }
    },
    [editor],
  )

  return (
    <div className="node-editor" onKeyDown={handleKeyDown} tabIndex={0}>
      <NodePalette fetchNodeTypes={api.fetchNodeTypes} onNodeTypes={setAvailableTypes} latestEventLabel={latestEventLabel} />
      <section className="editor-stage">
        <Toolbar
          canUndo={editor.canUndo}
          canRedo={editor.canRedo}
          onUndo={editor.undo}
          onRedo={editor.redo}
          toPipelineGraph={editor.toPipelineGraph}
          fromPipelineGraph={editor.fromPipelineGraph}
          saveGraph={api.saveGraph}
          loadGraphs={api.loadGraphs}
          loadGraph={api.loadGraph}
          executeGraph={api.executeGraph}
        />
        <NodeEditorCanvas editor={editor} nodeTypesByName={nodeTypesByName} />
      </section>
      <PropertyPanel node={editor.selectedNode} onUpdate={editor.updateNodeData} />
    </div>
  )
}

export function NodeEditor({ latestEvent }: { latestEvent?: LuxEventEnvelope }) {
  return (
    <ReactFlowProvider>
      <NodeEditorInner latestEvent={latestEvent} />
    </ReactFlowProvider>
  )
}
