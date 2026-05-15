import { useState, useEffect, useCallback, useRef } from 'react';

export type CaptureStatus = 'idle' | 'starting' | 'streaming' | 'stopping' | 'error';

export interface CaptureSession {
  session_id: string;
  status: CaptureStatus;
  width?: number;
  height?: number;
}

export function useCapture() {
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [status, setStatus] = useState<CaptureStatus>('idle');
  const [error, setError] = useState<string | null>(null);
  const [sessionInfo, setSessionInfo] = useState<CaptureSession | null>(null);
  const pollIntervalRef = useRef<number | null>(null);

  const clearPoll = useCallback(() => {
    if (pollIntervalRef.current !== null) {
      window.clearInterval(pollIntervalRef.current);
      pollIntervalRef.current = null;
    }
  }, []);

  const getStatus = useCallback(async (id: string) => {
    try {
      const response = await fetch(`/api/unity/capture/sessions/${id}`);
      if (!response.ok) {
        throw new Error(`Failed to get status: ${response.statusText}`);
      }
      const data = await response.json();
      setSessionInfo(data);
      setStatus(data.status);
      return data;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus('error');
      clearPoll();
      return null;
    }
  }, [clearPoll]);

  const startCapture = useCallback(async (projectPath: string, width: number, height: number, fps: number) => {
    try {
      setError(null);
      setStatus('starting');
      
      const response = await fetch('/api/unity/capture/sessions', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({ project_path: projectPath, width, height, fps }),
      });

      if (!response.ok) {
        throw new Error(`Failed to start capture: ${response.statusText}`);
      }

      const data = await response.json();
      setSessionId(data.session_id);
      setStatus(data.status);
      setSessionInfo(data);
      
      return data.session_id;
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus('error');
      return null;
    }
  }, []);

  const stopCapture = useCallback(async () => {
    if (!sessionId) return;
    
    try {
      setStatus('stopping');
      clearPoll();
      
      const response = await fetch(`/api/unity/capture/sessions/${sessionId}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw new Error(`Failed to stop capture: ${response.statusText}`);
      }

      setSessionId(null);
      setSessionInfo(null);
      setStatus('idle');
    } catch (err) {
      const msg = err instanceof Error ? err.message : String(err);
      setError(msg);
      setStatus('error');
    }
  }, [sessionId, clearPoll]);

  // Auto-poll status when starting
  useEffect(() => {
    if (status === 'starting' && sessionId) {
      pollIntervalRef.current = window.setInterval(() => {
        getStatus(sessionId);
      }, 2000);
    } else {
      clearPoll();
    }

    return clearPoll;
  }, [status, sessionId, getStatus, clearPoll]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      clearPoll();
      // We don't automatically stop the capture on unmount to allow background streaming,
      // but we could if that's the desired behavior.
    };
  }, [clearPoll]);

  return {
    sessionId,
    status,
    error,
    sessionInfo,
    startCapture,
    stopCapture,
    getStatus
  };
}
