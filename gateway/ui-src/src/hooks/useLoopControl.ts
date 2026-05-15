import { useState, useEffect, useCallback } from 'react';

export interface LoopStatus {
  current_phase: string;
  iteration: number;
  is_running: boolean;
  last_error: string | null;
}

interface ProgressSummaryResponse {
  spec: unknown;
  kanban: unknown;
  loop: {
    state: string;
    iteration: number | null;
  };
}

export interface UseLoopControlResult {
  status: LoopStatus | null;
  loading: boolean;
  error: string | null;
  recordPlayStarted: () => Promise<void>;
  submitFeedback: (rating: number, text: string, issues: string[]) => Promise<void>;
  refresh: () => Promise<void>;
}

function withProject(path: string, projectPath: string): string {
  const separator = path.includes('?') ? '&' : '?';
  return `${path}${separator}project_path=${encodeURIComponent(projectPath)}`;
}

async function fetchJson<T>(path: string): Promise<T> {
  const response = await fetch(path);
  if (!response.ok) {
    const body = await response.text();
    throw new Error(`${path} failed: ${response.status} ${body}`);
  }
  return response.json() as Promise<T>;
}

async function postJson(path: string, body: unknown): Promise<void> {
  const response = await fetch(path, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    const text = await response.text();
    throw new Error(`${path} failed: ${response.status} ${text}`);
  }
}

export function useLoopControl(projectPath?: string | null): UseLoopControlResult {
  const [status, setStatus] = useState<LoopStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    if (!projectPath) {
      setStatus(null);
      setLoading(false);
      setError(null);
      return;
    }

    try {
      setLoading(true);
      const data = await fetchJson<ProgressSummaryResponse>(
        withProject('/api/lux/progress/summary', projectPath),
      );
      const phase = data.loop.state;
      const iteration = data.loop.iteration ?? 0;
      const isRunning = phase !== 'Idle' && phase !== 'Error';
      setStatus({
        current_phase: phase,
        iteration,
        is_running: isRunning,
        last_error: null,
      });
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load loop status');
    } finally {
      setLoading(false);
    }
  }, [projectPath]);

  useEffect(() => {
    void refresh();
    const interval = window.setInterval(() => void refresh(), 5000);
    return () => window.clearInterval(interval);
  }, [refresh]);

  const recordPlayStarted = useCallback(async () => {
    await postJson('/api/lux/loop/play-started', {});
  }, []);

  const submitFeedback = useCallback(async (rating: number, text: string, issues: string[]) => {
    await postJson('/api/lux/loop/feedback', { rating, text, issues });
  }, []);

  return {
    status,
    loading,
    error,
    recordPlayStarted,
    submitFeedback,
    refresh,
  };
}
