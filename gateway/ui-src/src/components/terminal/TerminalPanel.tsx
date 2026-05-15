import { useCallback, useEffect, useRef } from 'react'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import { useTerminal } from '../../hooks/useTerminal'
import { TerminalSessionList } from './TerminalSessionList'

export function TerminalPanel() {
  const terminalElementRef = useRef<HTMLDivElement | null>(null)
  const terminalRef = useRef<Terminal | null>(null)
  const fitAddonRef = useRef<FitAddon | null>(null)
  const activeSessionIdRef = useRef<string | null>(null)
  const renderedCountsRef = useRef<Map<string, number>>(new Map())
  const commandRef = useRef('')

  const {
    sessions,
    activeSessionId,
    activeSession,
    outputs,
    isConnected,
    error,
    setActiveSessionId,
    createSession,
    destroySession,
    sendInput,
    connect,
  } = useTerminal()

  const writePrompt = useCallback((term: Terminal, sessionId: string | null) => {
    const label = sessionId ? sessionId.slice(0, 8) : 'no-session'
    term.write(`\r\n\x1b[38;5;81mlux:${label}\x1b[0m $ `)
  }, [])

  const redrawCommand = useCallback((term: Terminal) => {
    const label = activeSessionIdRef.current ? activeSessionIdRef.current.slice(0, 8) : 'no-session'
    term.write(`\r\x1b[2K\x1b[38;5;81mlux:${label}\x1b[0m $ ${commandRef.current}`)
  }, [])

  const handleCreateSession = useCallback(() => {
    void createSession().catch(err => {
      terminalRef.current?.writeln(`\r\n\x1b[31mFailed to create session: ${err instanceof Error ? err.message : String(err)}\x1b[0m`)
    })
  }, [createSession])

  const handleDestroySession = useCallback((sessionId: string) => {
    void destroySession(sessionId).catch(err => {
      terminalRef.current?.writeln(`\r\n\x1b[31mFailed to destroy session: ${err instanceof Error ? err.message : String(err)}\x1b[0m`)
    })
  }, [destroySession])

  const handleCopy = useCallback(() => {
    const term = terminalRef.current
    const selection = term?.getSelection()
    if (!selection) return
    void navigator.clipboard?.writeText(selection)
  }, [])

  const handlePaste = useCallback(() => {
    void navigator.clipboard?.readText().then(text => {
      const term = terminalRef.current
      if (!term || !text) return
      const sanitized = text.replace(/[\r\n]+/g, ' ')
      commandRef.current += sanitized
      term.write(sanitized)
    })
  }, [])

  useEffect(() => {
    activeSessionIdRef.current = activeSessionId
  }, [activeSessionId])

  useEffect(() => {
    if (!terminalElementRef.current || terminalRef.current) return

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      scrollback: 5000,
      fontFamily: 'JetBrains Mono, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 13,
      theme: {
        background: '#0b0f17',
        foreground: '#f5f7fb',
        cursor: '#38bdf8',
        selectionBackground: '#334155',
        black: '#0b0f17',
        red: '#f87171',
        green: '#34d399',
        yellow: '#fbbf24',
        blue: '#38bdf8',
        magenta: '#c084fc',
        cyan: '#22d3ee',
        white: '#f5f7fb',
      },
    })
    const fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(terminalElementRef.current)
    fitAddon.fit()
    terminalRef.current = term
    fitAddonRef.current = fitAddon

    term.writeln('\x1b[38;5;81mLUX Terminal Panel\x1b[0m')
    term.writeln('Create or select a session, then enter allowed Lux commands.')
    writePrompt(term, activeSessionIdRef.current)

    const dataDisposable = term.onData((data) => {
      const sessionId = activeSessionIdRef.current
      if (data === '\r') {
        const input = commandRef.current.trim()
        commandRef.current = ''
        term.write('\r\n')
        if (!sessionId) {
          term.writeln('\x1b[33mCreate a terminal session first.\x1b[0m')
          writePrompt(term, sessionId)
          return
        }
        if (!input) {
          writePrompt(term, sessionId)
          return
        }
        if (!sendInput(sessionId, input)) {
          term.writeln('\x1b[31mUnable to send input: WebSocket disconnected.\x1b[0m')
          writePrompt(term, sessionId)
        }
        return
      }

      if (data === '\u0003') {
        commandRef.current = ''
        term.write('^C')
        writePrompt(term, sessionId)
        return
      }

      if (data === '\u007F') {
        if (commandRef.current.length > 0) {
          commandRef.current = commandRef.current.slice(0, -1)
          term.write('\b \b')
        }
        return
      }

      if (data === '\x1b[3~') {
        return
      }

      if (data >= ' ') {
        commandRef.current += data
        term.write(data)
      }
    })

    const resize = () => fitAddon.fit()
    window.addEventListener('resize', resize)

    return () => {
      window.removeEventListener('resize', resize)
      dataDisposable.dispose()
      term.dispose()
      terminalRef.current = null
      fitAddonRef.current = null
    }
  }, [sendInput, writePrompt])

  useEffect(() => {
    fitAddonRef.current?.fit()
  }, [sessions.length, activeSessionId])

  useEffect(() => {
    const term = terminalRef.current
    if (!term) return
    term.clear()
    commandRef.current = ''
    term.writeln('\x1b[38;5;81mLUX Terminal Panel\x1b[0m')
    if (activeSession) {
      term.writeln(`Session ${activeSession.sessionId} · ${activeSession.status}`)
      const sessionOutput = outputs.get(activeSession.sessionId) ?? []
      sessionOutput.forEach(output => term.write(output.data))
      renderedCountsRef.current.set(activeSession.sessionId, sessionOutput.length)
    } else {
      term.writeln('No active terminal session.')
    }
    writePrompt(term, activeSessionId)
  }, [activeSession, activeSessionId, outputs, writePrompt])

  useEffect(() => {
    const term = terminalRef.current
    if (!term || !activeSessionId) return
    const sessionOutput = outputs.get(activeSessionId) ?? []
    const renderedCount = renderedCountsRef.current.get(activeSessionId) ?? 0
    if (sessionOutput.length <= renderedCount) return

    const pending = sessionOutput.slice(renderedCount)
    if (commandRef.current.length > 0) {
      term.write('\r\n')
    }
    pending.forEach(output => term.write(output.data))
    renderedCountsRef.current.set(activeSessionId, sessionOutput.length)
    writePrompt(term, activeSessionId)
    if (commandRef.current.length > 0) {
      redrawCommand(term)
    }
  }, [activeSessionId, outputs, redrawCommand, writePrompt])

  return (
    <div className="terminal-panel">
      <style>{terminalPanelStyles}</style>
      <TerminalSessionList
        sessions={sessions}
        activeSessionId={activeSessionId}
        isConnected={isConnected}
        onSelectSession={setActiveSessionId}
        onCreateSession={handleCreateSession}
        onDestroySession={handleDestroySession}
      />
      <section className="terminal-panel__main">
        <div className="terminal-panel__toolbar">
          <div>
            <p className="terminal-panel__eyebrow">WebSocket I/O</p>
            <h2>xterm.js terminal</h2>
          </div>
          <div className="terminal-panel__actions">
            {error && <span className="terminal-panel__error">{error}</span>}
            <button type="button" onClick={connect}>Connect</button>
            <button type="button" onClick={handleCopy}>Copy</button>
            <button type="button" onClick={handlePaste}>Paste</button>
          </div>
        </div>
        <div className="terminal-panel__frame" ref={terminalElementRef} />
      </section>
    </div>
  )
}

const terminalPanelStyles = `
.terminal-panel {
  display: flex;
  min-height: 0;
  height: 100%;
  background: var(--bg, #0b0f17);
  color: var(--text, #f5f7fb);
  overflow: hidden;
}

.terminal-session-list {
  display: flex;
  flex-direction: column;
  width: 240px;
  min-width: 220px;
  border-right: 1px solid var(--line, rgba(148, 163, 184, 0.2));
  background: var(--panel-strong, rgba(18, 26, 42, 0.94));
  padding: 14px;
  gap: 12px;
}

.terminal-session-list__header,
.terminal-panel__toolbar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.terminal-session-list__eyebrow,
.terminal-panel__eyebrow {
  margin: 0 0 4px;
  color: var(--muted, #9aa7bc);
  font-size: 0.72rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
}

.terminal-session-list h3,
.terminal-panel h2 {
  margin: 0;
}

.terminal-session-list__status {
  border: 1px solid var(--line, rgba(148, 163, 184, 0.2));
  border-radius: 999px;
  color: var(--muted, #9aa7bc);
  font-size: 0.72rem;
  padding: 3px 8px;
}

.terminal-session-list__status.is-connected {
  border-color: rgba(52, 211, 153, 0.45);
  color: #34d399;
}

.terminal-session-list__create,
.terminal-panel__actions button {
  border: 1px solid var(--line, rgba(148, 163, 184, 0.2));
  border-radius: 8px;
  background: rgba(56, 189, 248, 0.1);
  color: var(--text, #f5f7fb);
  cursor: pointer;
  padding: 8px 10px;
}

.terminal-session-list__create:hover,
.terminal-panel__actions button:hover {
  border-color: var(--blue, #38bdf8);
  color: var(--blue, #38bdf8);
}

.terminal-session-list__items {
  display: flex;
  flex-direction: column;
  gap: 8px;
  min-height: 0;
  overflow: auto;
}

.terminal-session-list__empty,
.terminal-panel__error {
  color: var(--muted, #9aa7bc);
  font-size: 0.85rem;
}

.terminal-panel__error {
  color: #f87171;
}

.terminal-session-list__item {
  display: flex;
  align-items: stretch;
  border: 1px solid var(--line, rgba(148, 163, 184, 0.2));
  border-radius: 10px;
  overflow: hidden;
}

.terminal-session-list__item.is-active {
  border-color: var(--blue, #38bdf8);
  background: rgba(56, 189, 248, 0.1);
}

.terminal-session-list__select,
.terminal-session-list__destroy {
  border: 0;
  background: transparent;
  color: inherit;
  cursor: pointer;
}

.terminal-session-list__select {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: flex-start;
  gap: 4px;
  min-width: 0;
  padding: 10px;
}

.terminal-session-list__name {
  color: var(--text, #f5f7fb);
  font-weight: 700;
}

.terminal-session-list__meta {
  color: var(--muted, #9aa7bc);
  font-size: 0.75rem;
}

.terminal-session-list__destroy {
  color: var(--muted, #9aa7bc);
  padding: 0 10px;
}

.terminal-session-list__destroy:hover {
  color: #f87171;
}

.terminal-panel__main {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
}

.terminal-panel__toolbar {
  border-bottom: 1px solid var(--line, rgba(148, 163, 184, 0.2));
  padding: 14px 18px;
}

.terminal-panel__actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.terminal-panel__frame {
  flex: 1;
  min-height: 0;
  padding: 12px;
  background: #0b0f17;
}

.terminal-panel__frame .xterm {
  height: 100%;
}
`
