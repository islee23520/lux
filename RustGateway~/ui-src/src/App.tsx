import { useState } from 'react'
import 'reactflow/dist/style.css'
import 'xterm/css/xterm.css'
import './App.css'
import { AITerminal } from './components/AITerminal'
import { NodeEditor } from './components/NodeEditor'
import type { ConnectionState, LuxEventEnvelope, ViewMode } from './types'
import { SessionManager } from './components/SessionManager'
import { RemoteViewer } from './components/RemoteViewer'
import { AITimeline } from './components/AITimeline'
function App() {
  const [activeView, setActiveView] = useState<ViewMode>('nodes')
  const [events, setEvents] = useState<LuxEventEnvelope[]>([])
  const [connectionState, setConnectionState] = useState<ConnectionState>('idle')
  const [activeSessionId, setActiveSessionId] = useState<string | null>(null)
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
        <button className={activeView === 'remote' ? 'active' : ''} onClick={() => setActiveView('remote')}>
          Remote
        </button>
        <button className={activeView === 'timeline' ? 'active' : ''} onClick={() => setActiveView('timeline')}>
          Timeline
        </button>
      </nav>

      <section className="workspace-card">
        {activeView === 'nodes' && (
          <NodeEditor latestEvent={latestEvent} />
        )}
        {activeView === 'terminal' && (
          <AITerminal onEvent={setEvents} onConnectionState={setConnectionState} />
        )}
        {activeView === 'remote' && (
          activeSessionId ? (
            <RemoteViewer 
              sessionId={activeSessionId} 
              onDisconnect={() => setActiveSessionId(null)} 
            />
          ) : (
            <SessionManager onSessionSelect={setActiveSessionId} />
          )
        )}
        {activeView === 'timeline' && (
          <AITimeline />
        )}
      </section>
    </main>
  )
}

export default App
