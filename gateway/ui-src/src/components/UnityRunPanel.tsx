import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useCapture as useCaptureSession } from '../hooks/useCapture';
import { useDashboard } from '../hooks/useDashboard';
import { useInputWs } from '../hooks/useInputWs';

const RESOLUTION_OPTIONS = [
  { label: '640 × 480', width: 640, height: 480 },
  { label: '1280 × 720', width: 1280, height: 720 },
  { label: '1920 × 1080', width: 1920, height: 1080 },
] as const;

const FPS_OPTIONS = [15, 30, 60] as const;

type Resolution = {
  width: number;
  height: number;
};

export const UnityRunPanel: React.FC = () => {
  const { projectInfo } = useDashboard();
  const capture = useCaptureSession();
  const { sessionId, status, error: captureError, sessionInfo, startCapture, stopCapture } = capture;
  const { isConnected, error: wsError, sendInput } = useInputWs(sessionId);

  const [targetFps, setTargetFps] = useState<number>(30);
  const [resolution, setResolution] = useState<Resolution>({ width: 1280, height: 720 });
  const [scanlinesEnabled, setScanlinesEnabled] = useState(true);
  const [receivedFps, setReceivedFps] = useState(0);
  const [streamError, setStreamError] = useState<string | null>(null);

  const captureAreaRef = useRef<HTMLDivElement>(null);
  const framesReceivedRef = useRef(0);

  const streamUrl = useMemo(() => {
    if (!sessionId) return null;
    return `/api/unity/capture/sessions/${sessionId}/stream`;
  }, [sessionId]);

  const displayResolution = useMemo(() => {
    const width = sessionInfo?.width ?? resolution.width;
    const height = sessionInfo?.height ?? resolution.height;
    return `${width} × ${height}`;
  }, [resolution.height, resolution.width, sessionInfo?.height, sessionInfo?.width]);

  const connectionError = captureError ?? wsError ?? streamError;
  const isStarting = status === 'starting';
  const isStreaming = status === 'streaming' && Boolean(sessionId && streamUrl);
  const isStopping = status === 'stopping';
  const isBusy = isStarting || isStopping;
  const showConnecting = isStarting || (isStreaming && !isConnected);

  useEffect(() => {
    framesReceivedRef.current = 0;
    setReceivedFps(0);
  }, [streamUrl]);

  useEffect(() => {
    if (!isStreaming || !streamUrl) return undefined;

    const controller = new AbortController();
    let previousFrameCount = framesReceivedRef.current;
    let previousSampleTime = performance.now();

    const sampleInterval = window.setInterval(() => {
      const now = performance.now();
      const elapsedSeconds = Math.max((now - previousSampleTime) / 1000, 0.001);
      const frameDelta = framesReceivedRef.current - previousFrameCount;
      setReceivedFps(Math.round(frameDelta / elapsedSeconds));
      previousFrameCount = framesReceivedRef.current;
      previousSampleTime = now;
    }, 1000);

    const countMjpegFrames = async () => {
      try {
        const response = await fetch(streamUrl, {
          cache: 'no-store',
          signal: controller.signal,
        });

        if (!response.ok) {
          throw new Error(`Stream FPS probe failed: ${response.status}`);
        }

        const reader = response.body?.getReader();
        if (!reader) {
          throw new Error('Stream FPS probe unavailable');
        }

        let previousByte: number | null = null;
        while (!controller.signal.aborted) {
          const { value, done } = await reader.read();
          if (done) break;
          if (!value) continue;

          for (const byte of value) {
            if (previousByte === 0xff && byte === 0xd9) {
              framesReceivedRef.current += 1;
            }
            previousByte = byte;
          }
        }
      } catch (error) {
        if (!controller.signal.aborted) {
          setStreamError(error instanceof Error ? error.message : String(error));
        }
      }
    };

    void countMjpegFrames();

    return () => {
      controller.abort();
      window.clearInterval(sampleInterval);
    };
  }, [isStreaming, streamUrl]);

  const handleStart = useCallback(() => {
    if (!projectInfo?.path) {
      setStreamError('No Unity project attached');
      return;
    }

    setStreamError(null);
    void startCapture(projectInfo.path, resolution.width, resolution.height, targetFps);
  }, [projectInfo?.path, resolution.height, resolution.width, startCapture, targetFps]);

  const handleStop = useCallback(() => {
    setStreamError(null);
    void stopCapture();
  }, [stopCapture]);

  const getNormalizedCoords = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    const rect = captureAreaRef.current?.getBoundingClientRect();
    if (!rect || rect.width === 0 || rect.height === 0) {
      return { x: 0, y: 0 };
    }

    const x = (event.clientX - rect.left) / rect.width;
    const y = (event.clientY - rect.top) / rect.height;

    return {
      x: Math.max(0, Math.min(1, x)),
      y: Math.max(0, Math.min(1, y)),
    };
  }, []);

  const canSendInput = isStreaming && isConnected;

  const handleMouseMove = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (!canSendInput) return;
    sendInput({ type: 'mouse_move', ...getNormalizedCoords(event) });
  }, [canSendInput, getNormalizedCoords, sendInput]);

  const handleMouseDown = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (!canSendInput) return;
    event.currentTarget.focus();
    sendInput({ type: 'mouse_down', ...getNormalizedCoords(event), button: event.button });
  }, [canSendInput, getNormalizedCoords, sendInput]);

  const handleMouseUp = useCallback((event: React.MouseEvent<HTMLDivElement>) => {
    if (!canSendInput) return;
    sendInput({ type: 'mouse_up', ...getNormalizedCoords(event), button: event.button });
  }, [canSendInput, getNormalizedCoords, sendInput]);

  const handleKeyDown = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!canSendInput) return;
    event.preventDefault();
    sendInput({ type: 'key_down', key_code: event.code });
  }, [canSendInput, sendInput]);

  const handleKeyUp = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (!canSendInput) return;
    event.preventDefault();
    sendInput({ type: 'key_up', key_code: event.code });
  }, [canSendInput, sendInput]);

  const handleResolutionChange = (value: string) => {
    const [width, height] = value.split('x').map(Number);
    setResolution({ width, height });
  };

  return (
    <section
      aria-label="Unity Run Panel"
      className="flex h-full w-full flex-col gap-4 bg-[var(--color-bg)] p-4 text-[var(--color-text)]"
    >
      <header className="flex items-center justify-between border-b border-[var(--color-line)] pb-4">
        <div>
          <h1 className="font-stencil m-0 text-[var(--text-title)]">UNITY RUN PANEL</h1>
          <p className="font-terminal m-0 mt-1 text-[var(--text-caption)] text-[var(--color-text-muted)]">
            {projectInfo?.path ?? 'Attach a Unity project to begin capture'}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {isStreaming && <span className="h-2 w-2 rounded-full bg-green-500 animate-flicker" aria-hidden="true" />}
          <span className="sys-tag">{isStreaming ? 'LIVE' : status.toUpperCase()}</span>
        </div>
      </header>

      <div className="corner-bracket flex min-h-0 flex-1 flex-col gap-3 border border-[var(--color-line)] bg-[var(--color-surface)] p-3">
        <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[var(--color-line)] pb-3">
          <div className="flex flex-wrap items-center gap-2">
            <button
              type="button"
              onClick={handleStart}
              disabled={isBusy || isStreaming || !projectInfo?.path}
              className="font-stencil border border-[var(--color-text)] bg-[var(--color-text)] px-4 py-2 text-[var(--text-caption)] text-[var(--color-bg)] transition-colors hover:bg-[var(--color-text-muted)] disabled:cursor-not-allowed disabled:opacity-40"
            >
              START
            </button>
            <button
              type="button"
              onClick={handleStop}
              disabled={!sessionId || isBusy}
              className="font-stencil border border-[var(--color-line)] px-4 py-2 text-[var(--text-caption)] transition-colors hover:bg-[var(--color-surface-raised)] disabled:cursor-not-allowed disabled:opacity-40"
            >
              STOP
            </button>
            <button
              type="button"
              onClick={() => setScanlinesEnabled((enabled) => !enabled)}
              className="sys-tag hover:border-[var(--color-line-strong)]"
              aria-pressed={scanlinesEnabled}
            >
              SCANLINES {scanlinesEnabled ? 'ON' : 'OFF'}
            </button>
          </div>

          <div className="flex flex-wrap items-center gap-2 font-terminal">
            <label className="sys-tag">
              TARGET
              <select
                value={targetFps}
                onChange={(event) => setTargetFps(Number(event.target.value))}
                disabled={status !== 'idle' && status !== 'error'}
                className="bg-transparent text-[var(--color-text)] outline-none disabled:opacity-50"
              >
                {FPS_OPTIONS.map((option) => (
                  <option key={option} value={option}>{option} FPS</option>
                ))}
              </select>
            </label>
            <span className="sys-tag">FPS {receivedFps}</span>
            <span className="sys-tag">LATENCY N/A</span>
          </div>
        </div>

        {connectionError && (
          <div className="font-terminal border border-red-500/40 bg-red-500/10 px-3 py-2 text-[var(--text-caption)] text-red-300" role="alert">
            CONNECTION LOST · {connectionError}
          </div>
        )}

        <div
          ref={captureAreaRef}
          className={`corner-bracket relative min-h-0 flex-1 overflow-hidden border border-[var(--color-line)] bg-[var(--color-bg)] ${scanlinesEnabled ? 'scanlines' : ''}`}
        >
          {streamUrl && isStreaming ? (
            <img
              src={streamUrl}
              alt="Unity Live Stream"
              draggable={false}
              onLoad={() => setStreamError(null)}
              onError={() => setStreamError('MJPEG stream disconnected')}
              className="h-full w-full select-none object-contain"
            />
          ) : (
            <div className="dot-grid-bg absolute inset-0 flex flex-col items-center justify-center gap-2 text-center text-[var(--color-text-muted)]">
              <span className="font-stencil text-[var(--text-body)]">
                {isStarting ? 'INITIALIZING CAPTURE' : 'NO ACTIVE CAPTURE SESSION'}
              </span>
              <span className="font-terminal text-[var(--text-caption)]">MJPEG stream standby</span>
            </div>
          )}

          {showConnecting && (
            <div className="pointer-events-none absolute inset-0 flex items-center justify-center bg-black/45">
              <span className="sys-tag animate-dot-pulse">CONNECTING</span>
            </div>
          )}

          <div
            aria-label="Unity capture input area"
            className="absolute inset-0 cursor-crosshair outline-none"
            role="button"
            tabIndex={0}
            onMouseMove={handleMouseMove}
            onMouseDown={handleMouseDown}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            onContextMenu={(event) => event.preventDefault()}
            onKeyDown={handleKeyDown}
            onKeyUp={handleKeyUp}
          />
        </div>

        <footer className="flex flex-wrap items-center justify-between gap-2 border-t border-[var(--color-line)] pt-3 font-terminal">
          <div className="flex flex-wrap items-center gap-2">
            <label className="sys-tag">
              RESOLUTION
              <select
                value={`${resolution.width}x${resolution.height}`}
                onChange={(event) => handleResolutionChange(event.target.value)}
                disabled={status !== 'idle' && status !== 'error'}
                className="bg-transparent text-[var(--color-text)] outline-none disabled:opacity-50"
              >
                {RESOLUTION_OPTIONS.map((option) => (
                  <option key={option.label} value={`${option.width}x${option.height}`}>{option.label}</option>
                ))}
              </select>
            </label>
            <span className="sys-tag">DISPLAY {displayResolution}</span>
            <span className="sys-tag">INPUT <span>{isConnected ? 'WS CONNECTED' : 'WS DISCONNECTED'}</span></span>
          </div>
          <span className="sys-tag max-w-full overflow-hidden text-ellipsis whitespace-nowrap">
            SESSION {sessionId ?? 'NONE'}
          </span>
        </footer>
      </div>
    </section>
  );
};
