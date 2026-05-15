import { useState, useRef, useCallback } from 'react'
import type { RemoteInputEvent, WebRTCConfig, SignalingMessage } from '../types'

// EXPERIMENTAL - Hidden by default: this hook is reachable only after the server roadmap flag remote_webrtc is enabled.
const INPUT_CHANNEL_LABEL = 'lux-remote-input'
const AI_COMMAND_CHANNEL_LABEL = 'lux-ai-commands'

function getGatewayToken(): string {
  return localStorage.getItem('lux-gateway-token') || ''
}

function authHeaders(): HeadersInit {
  const token = getGatewayToken()
  return token ? { 'x-lux-token': token } : {}
}

export function useWebRTC(sessionId: string) {
  const videoRef = useRef<HTMLVideoElement>(null)
  const [connectionState, setConnectionState] = useState<RTCPeerConnectionState>('new')
  const pcRef = useRef<RTCPeerConnection | null>(null)
  const wsRef = useRef<WebSocket | null>(null)
  const inputDataChannelRef = useRef<RTCDataChannel | null>(null)
  const aiCommandChannelRef = useRef<RTCDataChannel | null>(null)

  const disconnect = useCallback(() => {
    if (wsRef.current) {
      wsRef.current.close()
      wsRef.current = null
    }
    if (aiCommandChannelRef.current) {
      aiCommandChannelRef.current.close()
      aiCommandChannelRef.current = null
    }
    if (inputDataChannelRef.current) {
      inputDataChannelRef.current.close()
      inputDataChannelRef.current = null
    }
    if (pcRef.current) {
      pcRef.current.close()
      pcRef.current = null
    }
    if (videoRef.current) {
      videoRef.current.srcObject = null
    }
    setConnectionState('closed')
  }, [])

  const connect = useCallback(async () => {
    disconnect()
    setConnectionState('connecting')

    try {
      // 1. Fetch ICE config
      const response = await fetch(`/api/remote/sessions/${sessionId}/config`, {
        headers: authHeaders()
      })
      if (!response.ok) {
        throw new Error('Failed to fetch ICE config')
      }
      const config: WebRTCConfig = await response.json()

      // 2. Create RTCPeerConnection
      const pc = new RTCPeerConnection({
        iceServers: config.iceServers
      })
      pcRef.current = pc

      pc.onconnectionstatechange = () => {
        setConnectionState(pc.connectionState)
      }

      pc.ontrack = (event) => {
        if (videoRef.current && event.streams[0]) {
          videoRef.current.srcObject = event.streams[0]
        }
      }

      // 3. Create DataChannels with labels matching Unity expectations
      const inputDataChannel = pc.createDataChannel(INPUT_CHANNEL_LABEL)
      inputDataChannelRef.current = inputDataChannel

      const aiCommandChannel = pc.createDataChannel(AI_COMMAND_CHANNEL_LABEL)
      aiCommandChannelRef.current = aiCommandChannel

      // We want to receive video
      pc.addTransceiver('video', { direction: 'recvonly' })

      // 4. Connect to signaling WebSocket with token
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:'
      const token = getGatewayToken()
      const tokenParam = token ? `&token=${encodeURIComponent(token)}` : ''
      const wsUrl = `${protocol}//${window.location.host}/remote/signaling/${sessionId}?role=web${tokenParam}`
      const ws = new WebSocket(wsUrl)
      wsRef.current = ws

      ws.onopen = async () => {
        // 5. Create SDP offer
        const offer = await pc.createOffer()
        await pc.setLocalDescription(offer)

        // 6. Send offer via signaling
        const msg: SignalingMessage = {
          type: 'sdp-offer',
          payload: { sdp: offer.sdp || '' }
        }
        ws.send(JSON.stringify(msg))
      }

      ws.onmessage = async (event) => {
        const msg = JSON.parse(event.data) as SignalingMessage

        if (msg.type === 'sdp-answer') {
          // 7. Receive SDP answer from Unity
          await pc.setRemoteDescription(new RTCSessionDescription({
            type: 'answer',
            sdp: msg.payload.sdp
          }))
        } else if (msg.type === 'ice-candidate') {
          // 8. Exchange ICE candidates
          await pc.addIceCandidate(new RTCIceCandidate({
            candidate: msg.payload.candidate,
            sdpMid: msg.payload.sdpMid,
            sdpMLineIndex: msg.payload.sdpMLineIndex
          }))
        }
      }

      pc.onicecandidate = (event) => {
        if (event.candidate && ws.readyState === WebSocket.OPEN) {
          const msg: SignalingMessage = {
            type: 'ice-candidate',
            payload: {
              candidate: event.candidate.candidate,
              sdpMid: event.candidate.sdpMid || '',
              sdpMLineIndex: event.candidate.sdpMLineIndex || 0
            }
          }
          ws.send(JSON.stringify(msg))
        }
      }

      ws.onerror = () => {
        setConnectionState('failed')
      }

      ws.onclose = () => {
        if (pc.connectionState !== 'connected') {
          setConnectionState('disconnected')
        }
      }

    } catch (error) {
      console.error('WebRTC connection error:', error)
      setConnectionState('failed')
    }
  }, [sessionId, disconnect])

  // Send flat RemoteInputEvent JSON matching Unity's JsonUtility.FromJson<RemoteInputEvent>
  const sendInput = useCallback((event: RemoteInputEvent) => {
    if (inputDataChannelRef.current?.readyState === 'open') {
      inputDataChannelRef.current.send(JSON.stringify(event))
    }
  }, [])

  // Send AI command over dedicated lux-ai-commands DataChannel
  const sendAICommand = useCallback((command: string) => {
    if (aiCommandChannelRef.current?.readyState === 'open') {
      aiCommandChannelRef.current.send(JSON.stringify({ command }))
    }
  }, [])

  return {
    videoRef,
    connectionState,
    connect,
    disconnect,
    sendInput,
    sendAICommand
  }
}
