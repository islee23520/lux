import type { LuxFlowNode, LuxNodeData, PipelineParameter } from '../types'

export function PropertyPanel({
  node,
  onUpdate,
}: {
  node: LuxFlowNode | null
  onUpdate: (id: string, data: Partial<LuxNodeData>) => void
}) {
  if (!node) {
    return (
      <aside className="property-panel property-panel--empty">
        <p className="eyebrow">Inspector</p>
        <h2>No selection</h2>
        <p>Select a node to edit display name and parameters.</p>
      </aside>
    )
  }

  const updateParameter = (name: string, value: string) => {
    const parameters: PipelineParameter[] = node.data.parameters.map((parameter) =>
      parameter.name === name ? { ...parameter, value } : parameter,
    )
    onUpdate(node.id, { parameters })
  }

  return (
    <aside className="property-panel">
      <p className="eyebrow">Inspector</p>
      <label className="field-label">
        Display name
        <input value={node.data.label} onChange={(event) => onUpdate(node.id, { label: event.target.value })} />
      </label>
      <dl className="node-facts">
        <div>
          <dt>Type</dt>
          <dd>{node.data.pipelineType}</dd>
        </div>
        <div>
          <dt>Category</dt>
          <dd>{node.data.category}</dd>
        </div>
      </dl>
      <p className="property-description">{node.data.description}</p>

      <section className="property-section">
        <h3>Parameters</h3>
        {node.data.parameters.length === 0 ? <p>None</p> : null}
        {node.data.parameters.map((parameter) => (
          <label key={parameter.name} className="field-label">
            {parameter.name}
            <input value={parameter.value} onChange={(event) => updateParameter(parameter.name, event.target.value)} />
          </label>
        ))}
      </section>

      <section className="property-section">
        <h3>Input ports</h3>
        {node.data.inputPorts.map((port) => (
          <span key={port.name} className="port-chip port-chip--input">
            {port.name} · {port.dataType}
          </span>
        ))}
      </section>

      <section className="property-section">
        <h3>Output ports</h3>
        {node.data.outputPorts.map((port) => (
          <span key={port.name} className="port-chip port-chip--output">
            {port.name} · {port.dataType}
          </span>
        ))}
      </section>
    </aside>
  )
}
