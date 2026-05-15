import { useState, useEffect, useCallback, useRef } from 'react';

export type InputEventType = 'mouse_move' | 'mouse_down' | 'mouse_up' | 'key_down' | 'key_up';

export interface InputEvent {
  type: InputEventType;
  x?: number;
  y?: number;
  button?: number;
  key_code?: string;
}

export function useInputWs(sessionId: string | null) {
  const [isConnected, setIsConnected] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimeoutRef = useRef<number | null>(null);

  const connect = useCallback(() => {
    if (!sessionId) return;

    if (wsRef.current?.readyState === WebSocket.OPEN || wsRef.current?.readyState === WebSocket.CONNECTING) {
      return;
    }

    try {
      const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
      const wsUrl = `${protocol}//${window.location.host}/api/unity/capture/sessions/${sessionId}/input`;
      
      const ws = new WebSocket(wsUrl);

      ws.onopen = () => {
        setIsConnected(true);
        setError(null);
      };

      ws.onclose = () => {
        setIsConnected(false);
        wsRef.current = null;
        
        if (sessionId) {
          reconnectTimeoutRef.current = window.setTimeout(connect, 3000);
        }
      };

      ws.onerror = () => {
        setError('WebSocket connection error');
      };

      wsRef.current = ws;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [sessionId]);

  const disconnect = useCallback(() => {
    if (reconnectTimeoutRef.current !== null) {
      window.clearTimeout(reconnectTimeoutRef.current);
      reconnectTimeoutRef.current = null;
    }

    if (wsRef.current) {
      wsRef.current.close();
      wsRef.current = null;
    }
    
    setIsConnected(false);
  }, []);

  useEffect(() => {
    if (sessionId) {
      connect();
    } else {
      disconnect();
    }

    return () => {
      disconnect();
    };
  }, [sessionId, connect, disconnect]);

  const sendInput = useCallback((event: InputEvent) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(event));
      return true;
    }
    return false;
  }, []);

  return {
    isConnected,
    error,
    sendInput
  };
}
