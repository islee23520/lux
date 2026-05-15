import React, { useState } from 'react';
import { useLuxLoop, type DisplayPhase } from '../../hooks/useLuxLoop';

const PHASE_COLORS: Record<DisplayPhase, string> = {
  Idle: 'var(--color-text-muted)',
  Analyzing: 'var(--yellow, #facc15)',
  Building: 'var(--color-accent)',
  AwaitingPlay: 'var(--blue, #60a5fa)',
  AwaitingFeedback: 'var(--purple, #a78bfa)',
  ProcessingFeedback: 'var(--orange, #fb923c)',
  Finished: 'var(--green, #34d399)',
};

const PHASE_BADGE_CLASS: Record<DisplayPhase, string> = {
  Idle: 'badge-info',
  Analyzing: 'badge-warning',
  Building: 'badge-info',
  AwaitingPlay: 'badge-info',
  AwaitingFeedback: 'badge-warning',
  ProcessingFeedback: 'badge-warning',
  Finished: 'badge-success',
};

export const LoopStatusPanel: React.FC = () => {
  const {
    snapshot,
    phase,
    isApprovable,
    isCollectingFeedback,
    loading,
    error,
    actionLoading,
    actionError,
    feedbackSuccess,
    refresh,
    approve,
    reject,
    submitFeedback,
    clearActionError,
    clearFeedbackSuccess,
  } = useLuxLoop();

  const [showConfirmApprove, setShowConfirmApprove] = useState(false);
  const [showConfirmReject, setShowConfirmReject] = useState(false);
  const [rejectReason, setRejectReason] = useState('');
  const [feedbackRating, setFeedbackRating] = useState(3);
  const [feedbackNotes, setFeedbackNotes] = useState('');
  const [showErrorDetail, setShowErrorDetail] = useState(false);

  const handleApprove = async () => {
    await approve();
    setShowConfirmApprove(false);
  };

  const handleReject = async () => {
    await reject(rejectReason);
    setShowConfirmReject(false);
    setRejectReason('');
  };

  const handleFeedbackSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    await submitFeedback(feedbackRating, feedbackNotes);
    setFeedbackNotes('');
  };

  if (loading && !snapshot) {
    return (
      <section className="panel-card" aria-label="Loop status loading">
        <h3 className="panel-title">Loop Status</h3>
        <p style={{ color: 'var(--color-text-muted)' }}>Loading loop status...</p>
      </section>
    );
  }

  if (error && !snapshot) {
    return (
      <section className="panel-card" aria-label="Loop status error">
        <h3 className="panel-title">Loop Status</h3>
        <div className="badge badge-error" style={{ marginBottom: '8px' }}>Error</div>
        <p style={{ color: 'var(--red, #fb7185)' }}>{error}</p>
        <button className="btn" onClick={refresh} style={{ marginTop: '8px' }}>
          Retry
        </button>
      </section>
    );
  }

  if (!snapshot) {
    return (
      <section className="panel-card" aria-label="Loop status inactive">
        <h3 className="panel-title">Loop Status</h3>
        <p style={{ color: 'var(--color-text-muted)' }}>No active loop session.</p>
      </section>
    );
  }

  const iterationText = `Iteration ${snapshot.iteration} / ${snapshot.max_iterations}`;

  return (
    <section className="panel-card" aria-label="Loop status">
      <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: '12px' }}>
        <h3 className="panel-title" style={{ margin: 0, border: 'none', padding: 0 }}>Loop Status</h3>
        <span
          className={`badge ${PHASE_BADGE_CLASS[phase]}`}
          style={{
            backgroundColor: `${PHASE_COLORS[phase]}20`,
            color: PHASE_COLORS[phase],
            borderColor: PHASE_COLORS[phase],
          }}
        >
          {phase}
        </span>
      </div>

      <div style={{ display: 'flex', alignItems: 'center', gap: '8px', marginBottom: '12px' }}>
        <span
          className="status-dot"
          style={{
            backgroundColor: PHASE_COLORS[phase],
            boxShadow: `0 0 5px ${PHASE_COLORS[phase]}`,
          }}
        />
        <span style={{ fontSize: '0.9em', color: 'var(--color-text-muted)' }}>{iterationText}</span>
      </div>

      {snapshot.last_error && (
        <div
          style={{
            backgroundColor: 'rgba(251, 113, 133, 0.1)',
            border: '1px solid var(--red, #fb7185)',
            borderRadius: '4px',
            padding: '10px',
            marginBottom: '12px',
          }}
        >
          <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}>
            <span style={{ color: 'var(--red, #fb7185)', fontWeight: 'bold', fontSize: '0.85em' }}>Last Error</span>
            <button
              className="btn"
              style={{ padding: '2px 8px', fontSize: '0.75em' }}
              onClick={() => setShowErrorDetail(prev => !prev)}
            >
              {showErrorDetail ? 'Hide' : 'Details'}
            </button>
          </div>
          {showErrorDetail && (
            <p style={{ color: 'var(--red, #fb7185)', fontSize: '0.8em', marginTop: '6px', wordBreak: 'break-word' }}>
              {snapshot.last_error}
            </p>
          )}
        </div>
      )}

      {isApprovable && !isCollectingFeedback && (
        <div style={{ display: 'flex', gap: '8px', marginBottom: '12px' }}>
          {!showConfirmApprove ? (
            <button className="btn btn-primary" onClick={() => setShowConfirmApprove(true)} disabled={actionLoading}>
              Approve
            </button>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', flex: 1 }}>
              <p style={{ fontSize: '0.85em', margin: 0 }}>Are you sure you want to approve?</p>
              <div style={{ display: 'flex', gap: '6px' }}>
                <button className="btn btn-primary" onClick={handleApprove} disabled={actionLoading}>
                  Confirm
                </button>
                <button className="btn" onClick={() => setShowConfirmApprove(false)} disabled={actionLoading}>
                  Cancel
                </button>
              </div>
            </div>
          )}

          {!showConfirmReject ? (
            <button className="btn" onClick={() => setShowConfirmReject(true)} disabled={actionLoading}>
              Request Changes
            </button>
          ) : (
            <div style={{ display: 'flex', flexDirection: 'column', gap: '6px', flex: 1 }}>
              <p style={{ fontSize: '0.85em', margin: 0 }}>Request changes with reason:</p>
              <textarea
                className="btn"
                style={{
                  resize: 'vertical',
                  minHeight: '60px',
                  backgroundColor: 'var(--color-surface-raised)',
                  fontFamily: 'inherit',
                  padding: '6px 8px',
                }}
                value={rejectReason}
                onChange={e => setRejectReason(e.target.value)}
                placeholder="Enter reason..."
              />
              <div style={{ display: 'flex', gap: '6px' }}>
                <button className="btn btn-primary" onClick={handleReject} disabled={actionLoading}>
                  Confirm
                </button>
                <button className="btn" onClick={() => setShowConfirmReject(false)} disabled={actionLoading}>
                  Cancel
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {isCollectingFeedback && (
        <form onSubmit={handleFeedbackSubmit} style={{ marginBottom: '12px' }}>
          <div style={{ marginBottom: '8px' }}>
            <label style={{ display: 'block', fontSize: '0.85em', marginBottom: '4px', color: 'var(--color-text-muted)' }}>
              Rating
            </label>
            <select
              className="btn"
              style={{
                width: '100%',
                backgroundColor: 'var(--color-surface-raised)',
                cursor: 'pointer',
              }}
              value={feedbackRating}
              onChange={e => setFeedbackRating(Number(e.target.value))}
            >
              <option value={1}>1 - Poor</option>
              <option value={2}>2 - Fair</option>
              <option value={3}>3 - Good</option>
              <option value={4}>4 - Very Good</option>
              <option value={5}>5 - Excellent</option>
            </select>
          </div>
          <div style={{ marginBottom: '8px' }}>
            <label style={{ display: 'block', fontSize: '0.85em', marginBottom: '4px', color: 'var(--color-text-muted)' }}>
              Notes
            </label>
            <textarea
              className="btn"
              style={{
                width: '100%',
                resize: 'vertical',
                minHeight: '80px',
                backgroundColor: 'var(--color-surface-raised)',
                fontFamily: 'inherit',
                padding: '6px 8px',
              }}
              value={feedbackNotes}
              onChange={e => setFeedbackNotes(e.target.value)}
              placeholder="Enter your feedback..."
            />
          </div>
          <button type="submit" className="btn btn-primary" disabled={actionLoading}>
            {actionLoading ? 'Submitting...' : 'Submit Feedback'}
          </button>
        </form>
      )}

      {actionError && (
        <div
          style={{
            backgroundColor: 'rgba(251, 113, 133, 0.1)',
            border: '1px solid var(--red, #fb7185)',
            borderRadius: '4px',
            padding: '8px 10px',
            marginBottom: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px',
          }}
        >
          <span style={{ color: 'var(--red, #fb7185)', fontSize: '0.85em' }}>{actionError}</span>
          <button className="btn" style={{ padding: '2px 8px', fontSize: '0.75em' }} onClick={clearActionError}>
            Dismiss
          </button>
        </div>
      )}

      {feedbackSuccess && (
        <div
          style={{
            backgroundColor: 'rgba(52, 211, 153, 0.1)',
            border: '1px solid var(--green, #34d399)',
            borderRadius: '4px',
            padding: '8px 10px',
            marginBottom: '8px',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            gap: '8px',
          }}
        >
          <span style={{ color: 'var(--green, #34d399)', fontSize: '0.85em' }}>{feedbackSuccess}</span>
          <button className="btn" style={{ padding: '2px 8px', fontSize: '0.75em' }} onClick={clearFeedbackSuccess}>
            Dismiss
          </button>
        </div>
      )}

      <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
        <button className="btn" onClick={refresh} disabled={loading}>
          {loading ? 'Refreshing...' : 'Refresh'}
        </button>
        {snapshot.feedback_count > 0 && (
          <span style={{ fontSize: '0.8em', color: 'var(--color-text-muted)' }}>
            {snapshot.feedback_count} feedback entries
          </span>
        )}
      </div>
    </section>
  );
};
