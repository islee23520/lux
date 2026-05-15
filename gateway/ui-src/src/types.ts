import type { Edge, Node } from 'reactflow'

export type ViewMode = 'nodes' | 'terminal' | 'remote' | 'timeline' | 'dashboard'

export type ConnectionState = 'idle' | 'connecting' | 'connected' | 'error' | 'closed'

export type LuxEventEnvelope = {
  schema_version: number
  event_id: string
  category: string
  source: string
  session_id: string
  captured_at_utc: string
  payload: Record<string, unknown>
}

export interface PipelineGraph {
  schemaVersion: string
  id: string
  displayName: string
  nodes: PipelineNode[]
  edges: PipelineEdge[]
}

export interface PipelineNode {
  id: string
  type: string
  displayName: string
  inputPorts: PipelinePort[]
  outputPorts: PipelinePort[]
  parameters: PipelineParameter[]
}

export interface PipelinePort {
  name: string
  direction: 'Input' | 'Output'
  dataType: string
}

export interface PipelineEdge {
  id: string
  fromNodeId: string
  fromPortName: string
  toNodeId: string
  toPortName: string
}

export interface PipelineParameter {
  name: string
  value: string
}

export interface NodeTypeDefinition {
  type: string
  displayName: string
  description: string
  category: string
  inputPorts: { name: string; direction: string; dataType: string }[]
  outputPorts: { name: string; direction: string; dataType: string }[]
  parameters: { name: string; type: string; description: string }[]
}

export interface LuxNodeData {
  label: string
  pipelineType: string
  description: string
  category: string
  inputPorts: PipelinePort[]
  outputPorts: PipelinePort[]
  parameters: PipelineParameter[]
  status: 'ready' | 'linked' | 'queued' | 'running' | 'completed' | 'failed'
}

export type LuxFlowNode = Node<LuxNodeData>
export type LuxFlowEdge = Edge

export interface GraphSnapshot {
  nodes: LuxFlowNode[]
  edges: LuxFlowEdge[]
}

export interface SavedGraphSummary {
  id: string
  displayName: string
}

export interface GraphEditor {
  nodes: LuxFlowNode[]
  edges: LuxFlowEdge[]
  selectedNodeId: string | null
  selectedEdgeId: string | null
  selectedNode: LuxFlowNode | null
  canUndo: boolean
  canRedo: boolean
  onNodesChange: (changes: import('reactflow').NodeChange[]) => void
  onEdgesChange: (changes: import('reactflow').EdgeChange[]) => void
  addNode: (typeDef: NodeTypeDefinition, position?: { x: number; y: number }) => void
  removeNode: (id: string) => void
  updateNodeData: (id: string, data: Partial<LuxNodeData>) => void
  addEdge: (connection: import('reactflow').Connection) => void
  removeEdge: (id: string) => void
  setSelectedNodeId: (id: string | null) => void
  setSelectedEdgeId: (id: string | null) => void
  undo: () => void
  redo: () => void
  copy: () => void
  paste: () => void
  toPipelineGraph: () => PipelineGraph
  fromPipelineGraph: (graph: PipelineGraph) => void
}

export interface AvailableTool {
  type: string
  displayName: string
  description: string
  integrationMethod: string
  capabilities: string[]
  status: string
}

export interface ToolSession {
  id: string
  toolType: string
  status: 'connected' | 'disconnected' | 'error'
  createdAtUtc: string
  updatedAtUtc: string
  commandHistory: ToolCommandEntry[]
  lastOutput: string | null
}

export interface ToolCommandEntry {
  id: string
  command: string
  timestamp: string
  outputPreview: string | null
}

export interface ToolExecution {
  id: string
  toolSessionId: string
  command: string
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  createdAtUtc: string
  updatedAtUtc: string
  output: string | null
  error: string | null
}

export interface LuxSkill {
  name: string
  description: string
  toolType: string
}

export interface RemoteInputEvent {
  type: 'mouse-move' | 'mouse-down' | 'mouse-up' | 
        'key-down' | 'key-up' | 
        'touch-start' | 'touch-move' | 'touch-end' |
        'scroll'
  x: number        // normalized 0-1
  y: number        // normalized 0-1
  button?: number  // 0=left, 1=right, 2=middle
  key?: string     // key code
  touchId?: number
  deltaX?: number
  deltaY?: number
}

export interface WebRTCConfig {
  iceServers: Array<{
    urls: string[]
    username?: string
    credential?: string
  }>
}

export interface RemoteSession {
  id: string
  unityClientId: string
  webClientId: string | null
  status: 'waiting-for-unity' | 'waiting-for-web' | 'connected' | 'disconnected'
  stunUrls: string[]
  turnUrl: string | null
  createdAtUtc: string
  updatedAtUtc: string
}

export type SignalingMessage = 
  | { type: 'sdp-offer'; payload: { sdp: string } }
  | { type: 'sdp-answer'; payload: { sdp: string } }
  | { type: 'ice-candidate'; payload: { candidate: string; sdpMid: string; sdpMLineIndex: number } }

// AI Action Log types
export interface AiLogEntry {
  schema_version: number
  protocol: string
  id: string
  timestamp_utc: string
  source: string
  actor: string
  category: string
  action: string
  target: string
  message: string
  severity: string
  success: boolean
  metadata: Record<string, string>
}

export interface AiLogContextEntry {
  entry: AiLogEntry
  seconds_since_previous: number | null
  seconds_to_next: number | null
}
