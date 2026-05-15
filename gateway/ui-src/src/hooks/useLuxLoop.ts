import { useState, useEffect, useCallback } from 'react';

export type LoopState =
  | 'Idle'
  | 'Analyzing'
  | 'SpecRefining'
  | 'Building'
  | 'AwaitingPlay'
  | 'CollectingFeedback'
  | 'Updating'
  | { Paused: string };

export type DisplayPhase =
  | 'Idle'
  | 'Analyzing'
  | 'Building'
  | 'AwaitingPlay'
  | 'AwaitingFeedback'
  | 'ProcessingFeedback'
  | 'Finished';

export type ApprovalGate =
  | 'BeginAnalysis'
  | 'RefineSpec'
  | 'StartBuild'
  | 'StartPlay'
  | 'CollectFeedback'
  | 'UpdateSpec'
  | 'CompleteIteration';

export interface LoopSnapshot {
  state: LoopState;
  project_path: string;
  iteration: number;
  max_iterations: number;
  requires_user_approval: boolean;
  approval_gate: ApprovalGate | null;
  pending_state: LoopState | null;
  last_error: string | null;
  last_verification: unknown | null;
  last_ambiguity: unknown | null;
  active_ai_session: unknown | null;
  active_build_id: string | null;
  feedback_count: number;
}

export interface LuxLoopStatus {
  snapshot: LoopSnapshot | null;
  phase: DisplayPhase;
  isApprovable: boolean;
  isCollectingFeedback: boolean;
  loading: boolean;
  error: string | null;
  actionLoading: boolean;
  actionError: string | null;
  feedbackSuccess: string | null;
}

export interface LuxLoopActions {
  refresh: () => Promise<void>;
  approve: () => Promise<void>;
  reject: (reason: string) => Promise<void>;
  submitFeedback: (rating: number, notes: string) => Promise<void>;
  clearActionError: () => void;
  clearFeedbackSuccess: () => void;
}

function mapStateToDisplayPhase(state: LoopState): DisplayPhase {
  if (typeof state === 'object' && state !== null && 'Paused' in state) {
    return 'Idle';
  }
  switch (state) {
    case 'Idle':
      return 'Idle';
    case 'Analyzing':
    case 'SpecRefining':
      return 'Analyzing';
    case 'Building':
      return 'Building';
    case 'AwaitingPlay':
      return 'AwaitingPlay';
    case 'CollectingFeedback':
      return 'AwaitingFeedback';
    case 'Updating':
      return 'ProcessingFeedback';
    default:
      return 'Idle';
  }
}

function isApprovableState(snapshot: LoopSnapshot): boolean {
  return snapshot.requires_user_approval === true;
}

function isCollectingFeedbackState(snapshot: LoopSnapshot): boolean {
  return (
    snapshot.state === 'CollectingFeedback' ||
    snapshot.approval_gate === 'CollectFeedback'
  );
}

const POLL_INTERVAL_MS = 4000;

export function useLuxLoop(): LuxLoopStatus & LuxLoopActions {
  const [snapshot, setSnapshot] = useState<LoopSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [actionLoading, setActionLoading] = useState(false);
  const [actionError, setActionError] = useState<string | null>(null);
  const [feedbackSuccess, setFeedbackSuccess] = useState<string | null>(null);

  const fetchStatus = useCallback(async () => {
    try {
      const res = await fetch('/api/lux/loop/status');
      if (!res.ok) {
        if (res.status === 404) {
          setSnapshot(null);
          setError(null);
          return;
        }
        throw new Error(`Status ${res.status}: ${res.statusText}`);
      }
      const data: LoopSnapshot = await res.json();
      setSnapshot(data);
      setError(null);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Unknown error fetching loop status';
      setError(msg);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchStatus();
    const interval = setInterval(fetchStatus, POLL_INTERVAL_MS);
    return () => clearInterval(interval);
  }, [fetchStatus]);

  const approve = useCallback(async () => {
    setActionLoading(true);
    setActionError(null);
    try {
      const res = await fetch('/api/lux/loop/approve', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ approved: true }),
      });
      if (!res.ok) {
        throw new Error(`Approval failed: ${res.statusText}`);
      }
      const data: LoopSnapshot = await res.json();
      setSnapshot(data);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Approval failed';
      setActionError(msg);
    } finally {
      setActionLoading(false);
    }
  }, []);

  const reject = useCallback(async (reason: string) => {
    setActionLoading(true);
    setActionError(null);
    try {
      const res = await fetch('/api/lux/loop/feedback', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'rejection',
          reason: reason || 'Changes requested',
        }),
      });
      if (!res.ok) {
        throw new Error(`Rejection failed: ${res.statusText}`);
      }
      const data: LoopSnapshot = await res.json();
      setSnapshot(data);
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Rejection failed';
      setActionError(msg);
    } finally {
      setActionLoading(false);
    }
  }, []);

  const submitFeedback = useCallback(async (rating: number, notes: string) => {
    setActionLoading(true);
    setActionError(null);
    setFeedbackSuccess(null);
    try {
      const res = await fetch('/api/lux/loop/feedback', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          type: 'feedback',
          rating,
          notes,
        }),
      });
      if (!res.ok) {
        throw new Error(`Feedback failed: ${res.statusText}`);
      }
      const data: LoopSnapshot = await res.json();
      setSnapshot(data);
      setFeedbackSuccess('Feedback submitted successfully');
    } catch (err) {
      const msg = err instanceof Error ? err.message : 'Feedback submission failed';
      setActionError(msg);
    } finally {
      setActionLoading(false);
    }
  }, []);

  const clearActionError = useCallback(() => setActionError(null), []);
  const clearFeedbackSuccess = useCallback(() => setFeedbackSuccess(null), []);

  const phase = snapshot ? mapStateToDisplayPhase(snapshot.state) : 'Idle';

  return {
    snapshot,
    phase,
    isApprovable: snapshot ? isApprovableState(snapshot) : false,
    isCollectingFeedback: snapshot ? isCollectingFeedbackState(snapshot) : false,
    loading,
    error,
    actionLoading,
    actionError,
    feedbackSuccess,
    refresh: fetchStatus,
    approve,
    reject,
    submitFeedback,
    clearActionError,
    clearFeedbackSuccess,
  };
}
