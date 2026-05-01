import { useState } from 'react'
import 'reactflow/dist/style.css'
import 'xterm/css/xterm.css'
import './App.css'
import { AITerminal } from './components/AITerminal'
import { NodeEditor } from './components/NodeEditor'
import type { ConnectionState, LuxEventEnvelope, ViewMode } from './types'

function App() {
  const [activeView, setActiveView] = useState<ViewMode>('nodes')
  const [events, setEvents] = useState<LuxEventEnvelope[]>([])
  const [connectionState, setConnectionState] = useState<ConnectionState>('idle')

  const latestEvent = events[0]

  return (
    <main className="app-shell">
      <header className="app-header">
        <div>
          <p className="eyebrow">LUX Gateway</p>
          <h1>Pipeline console</h1>
        </div>
        <div className={`status-pill status-pill--${connectionState}`}>
          <span />
          {connectionState}
        </div>
      </header>

      <nav className="view-tabs" aria-label="Lux workspace views">
        <button className={activeView === 'nodes' ? 'active' : ''} onClick={() => setActiveView('nodes')}>
          Node editor
        </button>
        <button className={activeView === 'terminal' ? 'active' : ''} onClick={() => setActiveView('terminal')}>
          AI terminal
        </button>
      </nav>

      <section className="workspace-card">
        {activeView === 'nodes' ? (
          <NodeEditor latestEvent={latestEvent} />
        ) : (
          <AITerminal onEvent={setEvents} onConnectionState={setConnectionState} />
        )}
      </section>
    </main>
  )
}

export default App
