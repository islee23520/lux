import { useCallback, useEffect, useMemo, useRef, useState } from 'react';

export type BuildTarget = 'WebGL';

export type BuildStatus =
  | 'Queued'
  | 'Running'
  | 'Succeeded'
  | 'Cancelled'
  | { Failed: string };

export interface BuildJob {
  build_id: string;
  project_path: string;
  target: BuildTarget;
  status: BuildStatus;
  progress: number;
  started_at: string | null;
  completed_at: string | null;
  artifact_path: string | null;
  log: string[];
  error: string | null;
}

export type PlayEventType =
  | 'Action'
  | 'Decision'
  | 'Trigger'
  | 'Death'
  | 'LevelComplete'
  | 'LevelStart'
  | 'ItemCollect'
  | 'Damage'
  | 'MenuOpen'
  | 'MenuClose'
  | 'CutsceneStart'
  | 'CutsceneEnd'
  | 'Save'
  | 'Load'
  | { Custom: string };

export interface PlayEvent {
  session_id: string;
  timestamp: string;
  event_type: PlayEventType;
  payload: unknown;
  player_id: string | null;
  game_state: unknown | null;
  sequence: number;
}

export interface PlayerFeedbackInput {
  projectPath: string;
  sessionId: string;
  rating: number | null;
  text: string;
  issues: string[];
}

interface TriggerBuildResponse {
  buildId?: string;
  build_id?: string;
  job: BuildJob;
}

interface EventEnvelope {
  payload?: unknown;
  summary?: string | null;
  captured_at_utc?: string;
  session_id?: string;
}

interface LuxEventMessage {
  event?: unknown;
  timestamp?: string;
  source?: string;
}

type WsMessage = EventEnvelope | LuxEventMessage;

const MAX_EVENTS = 100;

function assertOk(response: Response, action: string): void {
  if (!response.ok) {
    throw new Error(`${action} failed: ${response.status}`);
  }
}

async function fetchWithFallback(primary: string, fallback: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(primary, init);
  if (response.status !== 404) {
    return response;
  }
  return fetch(fallback, init);
}

function buildStatusRoute(buildId: string): string {
  return `/api/lux/build/jobs/${encodeURIComponent(buildId)}/status`;
}

function buildStatusFallbackRoute(buildId: string): string {
  return `/api/lux/build/status/${encodeURIComponent(buildId)}`;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isPlayEvent(value: unknown): value is PlayEvent {
  return (
    isRecord(value) &&
    typeof value.session_id === 'string' &&
    typeof value.timestamp === 'string' &&
    'event_type' in value &&
    typeof value.sequence === 'number'
  );
}

function extractTypedEvent(message: WsMessage): { type: string; data: unknown } | null {
  if ('event' in message && isRecord(message.event)) {
    const event = message.event;
    if (typeof event.type === 'string') {
      return { type: event.type, data: event.data };
    }
  }

  if ('payload' in message && isRecord(message.payload)) {
    const payload = message.payload;
    if (typeof payload.type === 'string') {
      return { type: payload.type, data: payload.data ?? payload };
    }
    if (typeof payload.kind === 'string') {
      return { type: payload.kind, data: payload };
    }
  }

  return null;
}

function eventFromWsData(data: unknown): PlayEvent | null {
  if (isPlayEvent(data)) {
    return data;
  }
  if (isRecord(data) && isPlayEvent(data.event)) {
    return data.event;
  }
  return null;
}

function uniqueEvents(events: PlayEvent[]): PlayEvent[] {
  const seen = new Set<string>();
  return events.filter(event => {
    const key = `${event.session_id}:${event.sequence}:${event.timestamp}`;
    if (seen.has(key)) {
      return false;
    }
    seen.add(key);
    return true;
  });
}

export function buildStatusLabel(status: BuildStatus): string {
  return typeof status === 'string' ? status : `Failed: ${status.Failed}`;
}

export function isPlayableBuild(job: BuildJob): boolean {
  return job.status === 'Succeeded';
}

export function usePlayer() {
  const [builds, setBuilds] = useState<BuildJob[]>([]);
  const [events, setEvents] = useState<PlayEvent[]>([]);
  const [selectedBuildId, setSelectedBuildId] = useState<string | null>(null);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [eventsLoading, setEventsLoading] = useState(false);
  const [submittingFeedback, setSubmittingFeedback] = useState(false);
  const [triggeringBuild, setTriggeringBuild] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [wsConnected, setWsConnected] = useState(false);
  const wsRef = useRef<WebSocket | null>(null);

  const selectedBuild = useMemo(
    () => builds.find(build => build.build_id === selectedBuildId) ?? null,
    [builds, selectedBuildId],
  );

  const refreshBuilds = useCallback(async () => {
    setLoading(true);
    try {
      const response = await fetchWithFallback('/api/lux/build/jobs', '/api/lux/build/list');
      assertOk(response, 'Fetch builds');
      const jobs = (await response.json()) as BuildJob[];
      setBuilds(jobs);
      setSelectedBuildId(current => {
        if (current && jobs.some(job => job.build_id === current)) {
          return current;
        }
        return jobs.find(isPlayableBuild)?.build_id ?? jobs[0]?.build_id ?? null;
      });
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const getBuildStatus = useCallback(async (buildId: string): Promise<BuildJob> => {
    const response = await fetchWithFallback(buildStatusRoute(buildId), buildStatusFallbackRoute(buildId));
    assertOk(response, 'Fetch build status');
    return response.json() as Promise<BuildJob>;
  }, []);

  const triggerBuild = useCallback(async (projectPath?: string): Promise<BuildJob> => {
    setTriggeringBuild(true);
    try {
      const init: RequestInit = {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ projectPath }),
      };
      const response = await fetchWithFallback('/api/lux/build/trigger', '/api/lux/build/start', init);
      assertOk(response, 'Trigger build');
      const result = (await response.json()) as TriggerBuildResponse;
      setBuilds(current => [result.job, ...current.filter(job => job.build_id !== result.job.build_id)]);
      setSelectedBuildId(result.job.build_id);
      setError(null);
      return result.job;
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setTriggeringBuild(false);
    }
  }, []);

  const refreshEvents = useCallback(async (projectPath?: string, sessionId?: string | null) => {
    if (!projectPath || !sessionId) {
      setEvents([]);
      return;
    }

    setEventsLoading(true);
    try {
      const params = new URLSearchParams({ project_path: projectPath, limit: String(MAX_EVENTS) });
      const response = await fetchWithFallback(
        `/api/lux/play/events?session_id=${encodeURIComponent(sessionId)}&${params.toString()}`,
        `/api/lux/play/sessions/${encodeURIComponent(sessionId)}/events?${params.toString()}`,
      );
      assertOk(response, 'Fetch play events');
      const playEvents = (await response.json()) as PlayEvent[];
      setEvents(playEvents.slice(-MAX_EVENTS));
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setEventsLoading(false);
    }
  }, []);

  const submitFeedback = useCallback(async (feedback: PlayerFeedbackInput) => {
    setSubmittingFeedback(true);
    try {
      const response = await fetch('/api/lux/play/feedback', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          project_path: feedback.projectPath,
          session_id: feedback.sessionId,
          rating: feedback.rating,
          text: feedback.text.trim() || null,
          issues: feedback.issues,
        }),
      });
      assertOk(response, 'Submit feedback');
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      throw err;
    } finally {
      setSubmittingFeedback(false);
    }
  }, []);

  useEffect(() => {
    refreshBuilds();
  }, [refreshBuilds]);

  useEffect(() => {
    if (!selectedBuildId) {
      setSelectedSessionId(null);
      return;
    }
    setSelectedSessionId(selectedBuildId);
  }, [selectedBuildId]);

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const ws = new WebSocket(`${protocol}//${window.location.host}/events?role=player-ui`);
    wsRef.current = ws;

    ws.onopen = () => {
      setWsConnected(true);
    };

    ws.onclose = () => {
      setWsConnected(false);
      if (wsRef.current === ws) {
        wsRef.current = null;
      }
    };

    ws.onerror = () => {
      setWsConnected(false);
    };

    ws.onmessage = event => {
      try {
        const message = JSON.parse(event.data) as WsMessage;
        const typed = extractTypedEvent(message);
        if (!typed) {
          return;
        }

        if (typed.type === 'play:event') {
          const playEvent = eventFromWsData(typed.data);
          if (playEvent) {
            setEvents(current => uniqueEvents([...current, playEvent]).slice(-MAX_EVENTS));
          }
          return;
        }

        if (typed.type === 'build:complete') {
          refreshBuilds();
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      }
    };

    return () => {
      ws.close();
    };
  }, [refreshBuilds]);

  return {
    builds,
    events,
    selectedBuild,
    selectedBuildId,
    selectedSessionId,
    loading,
    eventsLoading,
    submittingFeedback,
    triggeringBuild,
    error,
    wsConnected,
    setSelectedBuildId,
    setSelectedSessionId,
    refreshBuilds,
    getBuildStatus,
    triggerBuild,
    refreshEvents,
    submitFeedback,
  };
}
