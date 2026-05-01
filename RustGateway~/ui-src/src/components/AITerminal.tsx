import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { Terminal } from 'xterm'
import { FitAddon } from 'xterm-addon-fit'
import type { ConnectionState, LuxEventEnvelope, ToolSession } from '../types'
import { useToolApi } from '../hooks/useToolApi'
import { ToolSelector } from './ToolSelector'
import { SkillPanel } from './SkillPanel'

export function AITerminal({
  onEvent,
  onConnectionState,
}: {
  onEvent: React.Dispatch<React.SetStateAction<LuxEventEnvelope[]>>
  onConnectionState: React.Dispatch<React.SetStateAction<ConnectionState>>
}) {
  const terminalRef = useRef<HTMLDivElement | null>(null)
  const termRef = useRef<Terminal | null>(null)
  const socketRef = useRef<WebSocket | null>(null)

  const [activeTool, setActiveTool] = useState<string>('claude-code')
  const [toolSessions, setToolSessions] = useState<Map<string, ToolSession>>(new Map())
  const { createSession, executeCommand, executeSkill } = useToolApi()

  const activeToolRef = useRef(activeTool)
  useEffect(() => {
    activeToolRef.current = activeTool
  }, [activeTool])

  const toolSessionsRef = useRef(toolSessions)
  useEffect(() => {
    toolSessionsRef.current = toolSessions
  }, [toolSessions])

  const endpoint = useMemo(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
    return `${protocol}//${window.location.host}/events?role=ui-terminal&client_id=lux-ui`
  }, [])

  const writePrompt = useCallback((term: Terminal) => {
    term.write(`\r\n\x1b[38;5;141m[${activeToolRef.current}]\x1b[0m > `)
  }, [])

  const handleSelectTool = useCallback(async (toolType: string) => {
    setActiveTool(toolType)
    const term = termRef.current
    if (term) {
      term.writeln(`\r\nSwitched to tool: ${toolType}`)
      writePrompt(term)
    }
    
    if (!toolSessionsRef.current.has(toolType)) {
      try {
        const session = await createSession(toolType)
        setToolSessions(prev => {
          const next = new Map(prev)
          next.set(toolType, session)
          return next
        })
      } catch (e) {
        term?.writeln(`\r\nFailed to create session for ${toolType}`)
        if (term) writePrompt(term)
      }
    }
  }, [createSession, writePrompt])

  const handleDispatchSkill = useCallback(async (skillName: string) => {
    const term = termRef.current
    if (!term) return
    
    term.writeln(`\r\nDispatching skill ${skillName} via ${activeToolRef.current}...`)
    try {
      const res = await executeSkill(activeToolRef.current, skillName, {})
      term.writeln(`\r\nSkill execution started: ${res.id}`)
    } catch (e) {
      term.writeln(`\r\nFailed to execute skill: ${e}`)
    }
    writePrompt(term)
  }, [executeSkill, writePrompt])

  const sendDemoEnvelope = useCallback(() => {
    const socket = socketRef.current
    const term = termRef.current
    if (!socket || socket.readyState !== WebSocket.OPEN) {
      term?.writeln('\r\nNot connected to /events yet.')
      return
    }

    const envelope: LuxEventEnvelope = {
      schema_version: 1,
      event_id: crypto.randomUUID(),
      category: 'tool',
      source: 'lux-ui',
      session_id: 'ui-terminal',
      captured_at_utc: new Date().toISOString(),
      payload: { kind: 'ai-tool-terminal', command: 'demo', tool: 'codex-image' },
    }

    socket.send(JSON.stringify(envelope))
    term?.writeln('\r\nSent demo AI tool envelope to gateway /events.')
  }, [])

  const connect = useCallback(() => {
    const term = termRef.current
    if (!term || socketRef.current?.readyState === WebSocket.OPEN) {
      return
    }

    onConnectionState('connecting')
    term.writeln(`\r\nConnecting ${endpoint}`)
    term.writeln('Browser WebSocket clients cannot set x-lux-token headers; embedded hosts may proxy or inject this connection.')

    const socket = new WebSocket(endpoint)
    socketRef.current = socket

    socket.addEventListener('open', () => {
      onConnectionState('connected')
      term.writeln('\r\nConnected to LUX gateway event stream.')
      writePrompt(term)
    })

    socket.addEventListener('message', (message) => {
      try {
        const envelope = JSON.parse(message.data) as LuxEventEnvelope
        onEvent((current) => [envelope, ...current].slice(0, 20))
        term.writeln(`\r\n[event] ${envelope.category} from ${envelope.source}`)
        
        if (envelope.category === 'tool-execute' || envelope.category === 'skill-dispatch') {
          term.writeln(`\r\n[${envelope.category}] ${JSON.stringify(envelope.payload)}`)
        }
      } catch {
        term.writeln(`\r\n[raw] ${String(message.data)}`)
      }
      writePrompt(term)
    })

    socket.addEventListener('close', () => {
      onConnectionState('closed')
      term.writeln('\r\nGateway event stream closed.')
      writePrompt(term)
    })

    socket.addEventListener('error', () => {
      onConnectionState('error')
      term.writeln('\r\nGateway connection error. Check token/proxy configuration and server status.')
      writePrompt(term)
    })
  }, [endpoint, onConnectionState, onEvent, writePrompt])

  useEffect(() => {
    if (!terminalRef.current || termRef.current) {
      return
    }

    const term = new Terminal({
      cursorBlink: true,
      convertEol: true,
      fontFamily: 'JetBrains Mono, SFMono-Regular, Menlo, Consolas, monospace',
      fontSize: 13,
      theme: {
        background: '#0b0d12',
        foreground: '#d7ddff',
        cursor: '#c084fc',
        selectionBackground: '#334155',
      },
    })
    const fitAddon = new FitAddon()
    term.loadAddon(fitAddon)
    term.open(terminalRef.current)
    fitAddon.fit()
    termRef.current = term

    term.writeln('LUX AI Tool Terminal')
    term.writeln('Commands: connect, demo, clear, tool list, tool use <tool>, tool status, run <cmd>, skill <name>, history')
    writePrompt(term)

    let command = ''
    const disposable = term.onData((data) => {
      if (data === '\r') {
        const input = command.trim()
        const inputLower = input.toLowerCase()
        command = ''
        if (inputLower === 'connect') connect()
        else if (inputLower === 'demo') sendDemoEnvelope()
        else if (inputLower === 'clear') term.clear()
        else if (inputLower === 'tool list') {
          term.writeln('\r\nAvailable tools: claude-code, openai-codex, opencode')
        }
        else if (inputLower.startsWith('tool use ')) {
          const toolType = inputLower.substring(9).trim()
          handleSelectTool(toolType)
          return
        }
        else if (inputLower === 'tool status') {
          const session = toolSessionsRef.current.get(activeToolRef.current)
          term.writeln(`\r\nStatus for ${activeToolRef.current}: ${session?.status || 'disconnected'}`)
        }
        else if (inputLower === 'history') {
          const session = toolSessionsRef.current.get(activeToolRef.current)
          if (session && session.commandHistory.length > 0) {
            term.writeln(`\r\nHistory for ${activeToolRef.current}:`)
            session.commandHistory.forEach(entry => {
              term.writeln(`\r\n[${entry.timestamp}] ${entry.command}`)
            })
          } else {
            term.writeln(`\r\nNo history for ${activeToolRef.current}`)
          }
        }
        else if (inputLower.startsWith('run ')) {
          const cmd = input.substring(4).trim()
          term.writeln(`\r\nRunning command: ${cmd}`)
          const session = toolSessionsRef.current.get(activeToolRef.current)
          executeCommand(activeToolRef.current, cmd, session?.id).then(res => {
            term.writeln(`\r\nExecution started: ${res.id}`)
            writePrompt(term)
          }).catch(e => {
            term.writeln(`\r\nFailed to execute command: ${e}`)
            writePrompt(term)
          })
          return
        }
        else if (inputLower.startsWith('skill ')) {
          const parts = input.substring(6).trim().split(' ')
          const skillName = parts[0]
          handleDispatchSkill(skillName)
          return
        }
        else if (input) term.writeln(`\r\nUnknown command: ${input}`)
        writePrompt(term)
        return
      }
      if (data === '\u007F') {
        if (command.length > 0) {
          command = command.slice(0, -1)
          term.write('\b \b')
        }
        return
      }
      if (data >= ' ') {
        command += data
        term.write(data)
      }
    })

    const resize = () => fitAddon.fit()
    window.addEventListener('resize', resize)

    return () => {
      window.removeEventListener('resize', resize)
      disposable.dispose()
      socketRef.current?.close()
      term.dispose()
      termRef.current = null
    }
  }, [connect, sendDemoEnvelope, writePrompt, handleSelectTool, handleDispatchSkill, executeCommand])

  return (
    <div className="terminal-view">
      <div className="terminal-sidebar">
        <ToolSelector activeTool={activeTool} onSelectTool={handleSelectTool} sessions={toolSessions} />
        <SkillPanel onDispatchSkill={handleDispatchSkill} />
      </div>
      <div className="terminal-main">
        <div className="terminal-toolbar">
          <div>
            <p className="eyebrow">AI tool bridge</p>
            <h2>Gateway /events terminal</h2>
          </div>
          <button onClick={connect}>Connect</button>
        </div>
        <div ref={terminalRef} className="terminal-frame" />
      </div>
    </div>
  )
}
