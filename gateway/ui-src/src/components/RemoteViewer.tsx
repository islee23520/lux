import { useEffect, useRef } from 'react'
import { useWebRTC } from '../hooks/useWebRTC'
import { RemoteCommandBar } from './RemoteCommandBar'

// EXPERIMENTAL - Hidden by default: RemoteViewer remains off normal UI navigation until .lux/roadmap.json opts into remote_webrtc.
interface RemoteViewerProps {
  sessionId: string
  onDisconnect: () => void
}

export function RemoteViewer({ sessionId, onDisconnect }: RemoteViewerProps) {
  const { videoRef, connectionState, connect, disconnect, sendInput, sendAICommand } = useWebRTC(sessionId)
  const containerRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    connect()
    return () => {
      disconnect()
    }
  }, [connect, disconnect])

  const handleMouseMove = (e: React.MouseEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height
    sendInput({ type: 'mouse-move', x, y })
  }

  const handleMouseDown = (e: React.MouseEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height
    sendInput({ type: 'mouse-down', x, y, button: e.button })
  }

  const handleMouseUp = (e: React.MouseEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height
    sendInput({ type: 'mouse-up', x, y, button: e.button })
  }

  const handleWheel = (e: React.WheelEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    const x = (e.clientX - rect.left) / rect.width
    const y = (e.clientY - rect.top) / rect.height
    sendInput({ type: 'scroll', x, y, deltaX: e.deltaX, deltaY: e.deltaY })
  }

  const handleKeyDown = (e: React.KeyboardEvent<HTMLVideoElement>) => {
    e.preventDefault()
    sendInput({ type: 'key-down', x: 0, y: 0, key: e.key })
  }

  const handleKeyUp = (e: React.KeyboardEvent<HTMLVideoElement>) => {
    e.preventDefault()
    sendInput({ type: 'key-up', x: 0, y: 0, key: e.key })
  }

  const handleTouchStart = (e: React.TouchEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    // e.preventDefault() // Cannot call preventDefault on passive event listener in React easily without ref, but we can try
    const rect = videoRef.current.getBoundingClientRect()
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i]
      const x = (touch.clientX - rect.left) / rect.width
      const y = (touch.clientY - rect.top) / rect.height
      sendInput({ type: 'touch-start', x, y, touchId: touch.identifier })
    }
  }

  const handleTouchMove = (e: React.TouchEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i]
      const x = (touch.clientX - rect.left) / rect.width
      const y = (touch.clientY - rect.top) / rect.height
      sendInput({ type: 'touch-move', x, y, touchId: touch.identifier })
    }
  }

  const handleTouchEnd = (e: React.TouchEvent<HTMLVideoElement>) => {
    if (!videoRef.current) return
    const rect = videoRef.current.getBoundingClientRect()
    for (let i = 0; i < e.changedTouches.length; i++) {
      const touch = e.changedTouches[i]
      const x = (touch.clientX - rect.left) / rect.width
      const y = (touch.clientY - rect.top) / rect.height
      sendInput({ type: 'touch-end', x, y, touchId: touch.identifier })
    }
  }

  // Add non-passive touch listeners to prevent scrolling
  useEffect(() => {
    const videoElement = videoRef.current
    if (!videoElement) return

    const preventDefault = (e: TouchEvent) => e.preventDefault()
    
    videoElement.addEventListener('touchstart', preventDefault, { passive: false })
    videoElement.addEventListener('touchmove', preventDefault, { passive: false })
    
    return () => {
      videoElement.removeEventListener('touchstart', preventDefault)
      videoElement.removeEventListener('touchmove', preventDefault)
    }
  }, [videoRef])

  return (
    <div className="remote-viewer" ref={containerRef}>
      <div className="remote-toolbar">
        <div className="remote-toolbar-left">
          <button onClick={onDisconnect} className="back-button">← Back</button>
          <span className="session-id">Session: {sessionId.substring(0, 8)}</span>
        </div>
        <div className={`status-pill status-pill--${connectionState}`}>
          <span />
          {connectionState}
        </div>
      </div>
      
      <div className="remote-video-container">
        <video 
          ref={videoRef} 
          autoPlay 
          playsInline 
          muted 
          tabIndex={0}
          className={`remote-video ${connectionState === 'connected' ? 'connected' : ''}`}
          onMouseMove={handleMouseMove}
          onMouseDown={handleMouseDown}
          onMouseUp={handleMouseUp}
          onWheel={handleWheel}
          onKeyDown={handleKeyDown}
          onKeyUp={handleKeyUp}
          onTouchStart={handleTouchStart}
          onTouchMove={handleTouchMove}
          onTouchEnd={handleTouchEnd}
          onContextMenu={(e) => e.preventDefault()}
        />
        
        {connectionState !== 'connected' && (
          <div className="connection-overlay">
            <div className="spinner"></div>
            <p>{connectionState === 'connecting' ? 'Connecting to Unity...' : 'Disconnected'}</p>
          </div>
        )}
      </div>
      
      <RemoteCommandBar onSendCommand={sendAICommand} />
    </div>
  )
}
