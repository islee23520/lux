import { useCallback, useMemo, useState } from 'react'
import {
  addEdge as addReactFlowEdge,
  applyEdgeChanges,
  applyNodeChanges,
  MarkerType,
  type Connection,
  type EdgeChange,
  type NodeChange,
  type XYPosition,
} from 'reactflow'
import type {
  GraphSnapshot,
  LuxFlowEdge,
  LuxFlowNode,
  LuxNodeData,
  NodeTypeDefinition,
  PipelineGraph,
  PipelineParameter,
  PipelinePort,
} from '../types'

const nodeDefaults = {
  type: 'luxPipeline',
}

const initialNodes: LuxFlowNode[] = [
  createNode(
    {
      type: 'UnityContext',
      displayName: 'Unity Context',
      description: 'Scene, selection, and editor state from the active project.',
      category: 'context',
      inputPorts: [],
      outputPorts: [{ name: 'context', direction: 'Output', dataType: 'UnityContext' }],
      parameters: [{ name: 'scope', type: 'string', description: 'Context scope' }],
    },
    { x: -60, y: 40 },
    'unity-context',
  ),
  createNode(
    {
      type: 'OutputDirectory',
      displayName: 'Output Directory',
      description: 'Package-local destination for generated sprites and masks.',
      category: 'context',
      inputPorts: [],
      outputPorts: [{ name: 'directory', direction: 'Output', dataType: 'DirectoryPath' }],
      parameters: [{ name: 'path', type: 'string', description: 'Output path' }],
    },
    { x: -60, y: 210 },
    'output-directory',
  ),
  createNode(
    {
      type: 'PromptTemplate',
      displayName: 'Prompt',
      description: 'Combines Unity context with reusable Codex Image prompts.',
      category: 'pipeline',
      inputPorts: [
        { name: 'context', direction: 'Input', dataType: 'UnityContext' },
        { name: 'output', direction: 'Input', dataType: 'DirectoryPath' },
      ],
      outputPorts: [{ name: 'prompt', direction: 'Output', dataType: 'Prompt' }],
      parameters: [{ name: 'template', type: 'string', description: 'Prompt template' }],
    },
    { x: 290, y: 115 },
    'prompt-template',
  ),
  createNode(
    {
      type: 'CodexGeneration',
      displayName: 'Generation',
      description: 'Queues an AI image generation job through Lux tooling.',
      category: 'pipeline',
      inputPorts: [{ name: 'prompt', direction: 'Input', dataType: 'Prompt' }],
      outputPorts: [{ name: 'image', direction: 'Output', dataType: 'Image' }],
      parameters: [{ name: 'model', type: 'string', description: 'Generation model' }],
    },
    { x: 650, y: 115 },
    'generation',
  ),
  createNode(
    {
      type: 'Segmentation',
      displayName: 'Segmentation',
      description: 'Separates subject, mask, and background layers.',
      category: 'post-process',
      inputPorts: [{ name: 'image', direction: 'Input', dataType: 'Image' }],
      outputPorts: [{ name: 'mask', direction: 'Output', dataType: 'Mask' }],
      parameters: [{ name: 'threshold', type: 'number', description: 'Mask threshold' }],
    },
    { x: 1000, y: 38 },
    'segmentation',
  ),
  createNode(
    {
      type: 'MaskPostProcessing',
      displayName: 'Export',
      description: 'Cleans masks and prepares Unity-ready assets.',
      category: 'post-process',
      inputPorts: [{ name: 'mask', direction: 'Input', dataType: 'Mask' }],
      outputPorts: [{ name: 'asset', direction: 'Output', dataType: 'UnityAsset' }],
      parameters: [{ name: 'format', type: 'string', description: 'Export format' }],
    },
    { x: 1000, y: 218 },
    'post-processing',
  ),
]

const initialEdges: LuxFlowEdge[] = [
  createEdge('context-prompt', 'unity-context', 'prompt-template', 'context', 'context', true),
  createEdge('output-prompt', 'output-directory', 'prompt-template', 'directory', 'output', false),
  createEdge('prompt-generation', 'prompt-template', 'generation', 'prompt', 'prompt', true),
  createEdge('generation-segmentation', 'generation', 'segmentation', 'image', 'image', false),
  createEdge('generation-post', 'generation', 'post-processing', 'image', 'mask', true),
]

function normalizePort(port: { name: string; direction: string; dataType: string }): PipelinePort {
  return {
    name: port.name,
    direction: port.direction === 'Output' ? 'Output' : 'Input',
    dataType: port.dataType,
  }
}

function createNode(typeDef: NodeTypeDefinition, position: XYPosition, id = `${typeDef.type}-${crypto.randomUUID()}`): LuxFlowNode {
  return {
    ...nodeDefaults,
    id,
    position,
    data: {
      label: typeDef.displayName,
      pipelineType: typeDef.type,
      description: typeDef.description,
      category: typeDef.category,
      inputPorts: typeDef.inputPorts.map(normalizePort),
      outputPorts: typeDef.outputPorts.map(normalizePort),
      parameters: typeDef.parameters.map<PipelineParameter>((parameter) => ({ name: parameter.name, value: '' })),
      status: 'ready',
    },
  }
}

function createEdge(
  id: string,
  source: string,
  target: string,
  sourceHandle: string,
  targetHandle: string,
  animated: boolean,
): LuxFlowEdge {
  return {
    id,
    source,
    target,
    sourceHandle,
    targetHandle,
    animated,
    markerEnd: { type: MarkerType.ArrowClosed, color: '#7dd3fc' },
  }
}

function cloneSnapshot(snapshot: GraphSnapshot): GraphSnapshot {
  return {
    nodes: snapshot.nodes.map((node) => ({ ...node, data: { ...node.data, parameters: [...node.data.parameters] } })),
    edges: snapshot.edges.map((edge) => ({ ...edge })),
  }
}

export function useGraphEditor() {
  const [nodes, setNodes] = useState<LuxFlowNode[]>(initialNodes)
  const [edges, setEdges] = useState<LuxFlowEdge[]>(initialEdges)
  const [undoStack, setUndoStack] = useState<GraphSnapshot[]>([])
  const [redoStack, setRedoStack] = useState<GraphSnapshot[]>([])
  const [clipboard, setClipboard] = useState<LuxFlowNode[]>([])
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null)
  const [selectedEdgeId, setSelectedEdgeId] = useState<string | null>(null)

  const snapshot = useCallback((): GraphSnapshot => cloneSnapshot({ nodes, edges }), [edges, nodes])

  const commit = useCallback(
    (next: GraphSnapshot) => {
      setUndoStack((current) => [...current.slice(-39), snapshot()])
      setRedoStack([])
      setNodes(next.nodes)
      setEdges(next.edges)
    },
    [snapshot],
  )

  const selectedNode = useMemo(
    () => nodes.find((node) => node.id === selectedNodeId) ?? null,
    [nodes, selectedNodeId],
  )

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      const removedIds = changes.flatMap((change) => (change.type === 'remove' ? [change.id] : []))
      if (removedIds.length > 0) {
        setUndoStack((current) => [...current.slice(-39), snapshot()])
        setRedoStack([])
      }
      setNodes((current) => applyNodeChanges(changes, current) as LuxFlowNode[])
      if (removedIds.length > 0) {
        setEdges((current) => current.filter((edge) => !removedIds.includes(edge.source) && !removedIds.includes(edge.target)))
      }
    },
    [snapshot],
  )

  const onEdgesChange = useCallback(
    (changes: EdgeChange[]) => {
      if (changes.some((change) => change.type === 'remove')) {
        setUndoStack((current) => [...current.slice(-39), snapshot()])
        setRedoStack([])
      }
      setEdges((current) => applyEdgeChanges(changes, current))
    },
    [snapshot],
  )

  const addNode = useCallback(
    (typeDef: NodeTypeDefinition, position: XYPosition = { x: 120, y: 120 }) => {
      commit({ nodes: [...nodes, createNode(typeDef, position)], edges })
    },
    [commit, edges, nodes],
  )

  const removeNode = useCallback(
    (id: string) => {
      commit({
        nodes: nodes.filter((node) => node.id !== id),
        edges: edges.filter((edge) => edge.source !== id && edge.target !== id),
      })
      setSelectedNodeId(null)
    },
    [commit, edges, nodes],
  )

  const removeEdge = useCallback(
    (id: string) => {
      commit({ nodes, edges: edges.filter((edge) => edge.id !== id) })
      setSelectedEdgeId(null)
    },
    [commit, edges, nodes],
  )

  const updateNodeData = useCallback(
    (id: string, data: Partial<LuxNodeData>) => {
      commit({
        nodes: nodes.map((node) => (node.id === id ? { ...node, data: { ...node.data, ...data } } : node)),
        edges,
      })
    },
    [commit, edges, nodes],
  )

  const addEdge = useCallback(
    (connection: Connection) => {
      if (!connection.source || !connection.target || !connection.sourceHandle || !connection.targetHandle) {
        return
      }
      const sourceNode = nodes.find((node) => node.id === connection.source)
      const targetNode = nodes.find((node) => node.id === connection.target)
      const sourcePort = sourceNode?.data.outputPorts.find((port) => port.name === connection.sourceHandle)
      const targetPort = targetNode?.data.inputPorts.find((port) => port.name === connection.targetHandle)
      if (!sourcePort || !targetPort || sourcePort.dataType !== targetPort.dataType) {
        return
      }
      const edge = createEdge(
        `${connection.source}-${connection.sourceHandle}-${connection.target}-${connection.targetHandle}`,
        connection.source,
        connection.target,
        connection.sourceHandle,
        connection.targetHandle,
        true,
      )
      commit({ nodes, edges: addReactFlowEdge(edge, edges) })
    },
    [commit, edges, nodes],
  )

  const undo = useCallback(() => {
    const previous = undoStack.at(-1)
    if (!previous) return
    setRedoStack((current) => [...current, snapshot()])
    setUndoStack((current) => current.slice(0, -1))
    setNodes(previous.nodes)
    setEdges(previous.edges)
  }, [snapshot, undoStack])

  const redo = useCallback(() => {
    const next = redoStack.at(-1)
    if (!next) return
    setUndoStack((current) => [...current, snapshot()])
    setRedoStack((current) => current.slice(0, -1))
    setNodes(next.nodes)
    setEdges(next.edges)
  }, [redoStack, snapshot])

  const copy = useCallback(() => {
    const selected = nodes.filter((node) => node.selected || node.id === selectedNodeId)
    setClipboard(selected.map((node) => ({ ...node, data: { ...node.data, parameters: [...node.data.parameters] } })))
  }, [nodes, selectedNodeId])

  const paste = useCallback(() => {
    if (clipboard.length === 0) return
    const pasted = clipboard.map((node) => ({
      ...node,
      id: `${node.data.pipelineType}-${crypto.randomUUID()}`,
      selected: true,
      position: { x: node.position.x + 42, y: node.position.y + 42 },
      data: { ...node.data, label: `${node.data.label} Copy`, parameters: [...node.data.parameters] },
    }))
    commit({ nodes: [...nodes.map((node) => ({ ...node, selected: false })), ...pasted], edges })
  }, [clipboard, commit, edges, nodes])

  const toPipelineGraph = useCallback((): PipelineGraph => {
    return {
      schemaVersion: '0.1',
      id: 'lux-graph',
      displayName: 'LUX CodexImage Pipeline',
      nodes: nodes.map((node) => ({
        id: node.id,
        type: node.data.pipelineType,
        displayName: node.data.label,
        inputPorts: node.data.inputPorts,
        outputPorts: node.data.outputPorts,
        parameters: node.data.parameters,
      })),
      edges: edges.map((edge) => ({
        id: edge.id,
        fromNodeId: edge.source,
        fromPortName: edge.sourceHandle ?? 'out',
        toNodeId: edge.target,
        toPortName: edge.targetHandle ?? 'in',
      })),
    }
  }, [edges, nodes])

  const fromPipelineGraph = useCallback(
    (graph: PipelineGraph) => {
      const nextNodes: LuxFlowNode[] = graph.nodes.map((node, index) => ({
        ...nodeDefaults,
        id: node.id,
        position: { x: 120 + (index % 3) * 330, y: 90 + Math.floor(index / 3) * 190 },
        data: {
          label: node.displayName,
          pipelineType: node.type,
          description: `${node.type} node loaded from ${graph.displayName}`,
          category: 'loaded',
          inputPorts: node.inputPorts,
          outputPorts: node.outputPorts,
          parameters: node.parameters,
          status: 'ready',
        },
      }))
      const nextEdges = graph.edges.map((edge) =>
        createEdge(edge.id, edge.fromNodeId, edge.toNodeId, edge.fromPortName, edge.toPortName, false),
      )
      commit({ nodes: nextNodes, edges: nextEdges })
    },
    [commit],
  )

  return {
    nodes,
    edges,
    selectedNodeId,
    selectedEdgeId,
    selectedNode,
    canUndo: undoStack.length > 0,
    canRedo: redoStack.length > 0,
    onNodesChange,
    onEdgesChange,
    addNode,
    removeNode,
    updateNodeData,
    addEdge,
    removeEdge,
    setSelectedNodeId,
    setSelectedEdgeId,
    undo,
    redo,
    copy,
    paste,
    toPipelineGraph,
    fromPipelineGraph,
  }
}
