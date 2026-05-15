import { useCallback, useEffect, useRef, useState } from 'react';

export interface CaptureSession {
  sessionId: string;
  streamUrl: string;
  inputWsUrl: string;
}

export interface UseCaptureSessionReturn {
  session: CaptureSession | null;
  start: (opts?: { width?: number; height?: number; fps?: number }) => Promise<void>;
  stop: () => Promise<void>;
  isStreaming: boolean;
  error: string | null;
  fps: number;
  latency: number;
}

interface CreateCaptureSessionResponse {
  session_id?: unknown;
  sessionId?: unknown;
  session?: {
    id?: unknown;
    session_id?: unknown;
    sessionId?: unknown;
  };
}

const FRAME_BOUNDARY = '--FRAME_BOUNDARY';

function extractSessionId(data: CreateCaptureSessionResponse): string | null {
  if (typeof data.session_id === 'string') return data.session_id;
  if (typeof data.sessionId === 'string') return data.sessionId;
  if (typeof data.session?.id === 'string') return data.session.id;
  if (typeof data.session?.session_id === 'string') return data.session.session_id;
  if (typeof data.session?.sessionId === 'string') return data.session.sessionId;
  return null;
}

function buildSession(id: string): CaptureSession {
  const streamUrl = `/api/unity/runs/${id}/stream`;
  const inputWsUrl = `ws://${window.location.host}/api/unity/runs/${id}/input`;

  return {
    sessionId: id,
    streamUrl,
    inputWsUrl,
  };
}

async function responseError(prefix: string, response: Response): Promise<Error> {
  const text = await response.text().catch(() => '');
  const detail = text.trim() || response.statusText || `HTTP ${response.status}`;
  return new Error(`${prefix}: ${detail}`);
}

export function useCaptureSession(): UseCaptureSessionReturn {
  const [session, setSession] = useState<CaptureSession | null>(null);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [fps, setFps] = useState(0);
  const [latency, setLatency] = useState(0);

  const sessionRef = useRef<CaptureSession | null>(null);
  const streamAbortRef = useRef<AbortController | null>(null);
  const frameCountRef = useRef(0);
  const lastFpsTimestampRef = useRef(0);
  const lastFrameTimestampRef = useRef<number | null>(null);

  const resetMetrics = useCallback(() => {
    frameCountRef.current = 0;
    lastFpsTimestampRef.current = performance.now();
    lastFrameTimestampRef.current = null;
    setFps(0);
    setLatency(0);
  }, []);

  const recordFrame = useCallback(() => {
    const now = performance.now();
    const lastFrameTimestamp = lastFrameTimestampRef.current;

    if (lastFrameTimestamp !== null) {
      setLatency(Math.round(now - lastFrameTimestamp));
    }

    lastFrameTimestampRef.current = now;
    frameCountRef.current += 1;

    const elapsed = now - lastFpsTimestampRef.current;
    if (elapsed >= 1000) {
      setFps(Math.round((frameCountRef.current * 1000) / elapsed));
      frameCountRef.current = 0;
      lastFpsTimestampRef.current = now;
    }
  }, []);

  const cleanupStreamMonitor = useCallback(() => {
    if (streamAbortRef.current) {
      streamAbortRef.current.abort();
      streamAbortRef.current = null;
    }
  }, []);

  const monitorStream = useCallback(async (captureSession: CaptureSession, controller: AbortController) => {
    try {
      const response = await fetch(captureSession.streamUrl, { signal: controller.signal });

      if (!response.ok) {
        throw await responseError('Failed to open capture stream', response);
      }

      if (!response.body) {
        throw new Error('Capture stream is not readable');
      }

      setIsStreaming(true);
      setError(null);

      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let pending = '';

      while (!controller.signal.aborted) {
        const { value, done } = await reader.read();

        if (done) {
          break;
        }

        pending += decoder.decode(value, { stream: true });

        let boundaryIndex = pending.indexOf(FRAME_BOUNDARY);
        while (boundaryIndex !== -1) {
          recordFrame();
          pending = pending.slice(boundaryIndex + FRAME_BOUNDARY.length);
          boundaryIndex = pending.indexOf(FRAME_BOUNDARY);
        }

        if (pending.length > FRAME_BOUNDARY.length * 2) {
          pending = pending.slice(-FRAME_BOUNDARY.length);
        }
      }

      if (!controller.signal.aborted && sessionRef.current?.sessionId === captureSession.sessionId) {
        setIsStreaming(false);
        setError('Capture stream connection lost');
      }
    } catch (err) {
      if (controller.signal.aborted) return;

      if (sessionRef.current?.sessionId === captureSession.sessionId) {
        setIsStreaming(false);
        setError(err instanceof Error ? err.message : String(err));
      }
    }
  }, [recordFrame]);

  const start = useCallback(async (opts?: { width?: number; height?: number; fps?: number }) => {
    cleanupStreamMonitor();
    sessionRef.current = null;
    setSession(null);
    setIsStreaming(false);
    setError(null);
    resetMetrics();

    try {
      const response = await fetch('/api/unity/runs', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify({
          width: opts?.width,
          height: opts?.height,
          fps: opts?.fps,
        }),
      });

      if (!response.ok) {
        throw await responseError('Failed to start capture session', response);
      }

      const data = (await response.json()) as CreateCaptureSessionResponse;
      const sessionId = extractSessionId(data);

      if (!sessionId) {
        throw new Error('Capture session response did not include a session id');
      }

      const nextSession = buildSession(sessionId);
      const controller = new AbortController();

      sessionRef.current = nextSession;
      streamAbortRef.current = controller;
      setSession(nextSession);
      void monitorStream(nextSession, controller);
    } catch (err) {
      cleanupStreamMonitor();
      sessionRef.current = null;
      setSession(null);
      setIsStreaming(false);
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [cleanupStreamMonitor, monitorStream, resetMetrics]);

  const stop = useCallback(async () => {
    const activeSession = sessionRef.current;

    cleanupStreamMonitor();
    sessionRef.current = null;
    setSession(null);
    setIsStreaming(false);
    resetMetrics();

    if (!activeSession) {
      return;
    }

    try {
      const response = await fetch(`/api/unity/runs/${activeSession.sessionId}`, {
        method: 'DELETE',
      });

      if (!response.ok) {
        throw await responseError('Failed to stop capture session', response);
      }

      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  }, [cleanupStreamMonitor, resetMetrics]);

  useEffect(() => {
    return () => {
      cleanupStreamMonitor();
    };
  }, [cleanupStreamMonitor]);

  return {
    session,
    start,
    stop,
    isStreaming,
    error,
    fps,
    latency,
  };
}
