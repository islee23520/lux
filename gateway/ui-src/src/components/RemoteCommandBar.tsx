import { useState } from 'react'

interface RemoteCommandBarProps {
  onSendCommand: (command: string) => void
}

export function RemoteCommandBar({ onSendCommand }: RemoteCommandBarProps) {
  const [command, setCommand] = useState('')

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault()
    if (command.trim()) {
      onSendCommand(command.trim())
      setCommand('')
    }
  }

  const handleQuickAction = (action: string) => {
    onSendCommand(action)
  }

  return (
    <div className="remote-command-bar">
      <div className="quick-actions">
        <button onClick={() => handleQuickAction('screenshot')}>Screenshot</button>
        <button onClick={() => handleQuickAction('compile')}>Compile</button>
        <button onClick={() => handleQuickAction('test')}>Test</button>
        <button onClick={() => handleQuickAction('play-pause')}>Play/Pause</button>
      </div>
      <form onSubmit={handleSubmit} className="command-form">
        <input
          type="text"
          value={command}
          onChange={(e) => setCommand(e.target.value)}
          placeholder="Enter AI command..."
          className="command-input"
        />
        <button type="submit" disabled={!command.trim()} className="execute-button">
          Send
        </button>
      </form>
    </div>
  )
}
