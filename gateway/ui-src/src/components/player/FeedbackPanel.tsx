import React, { useState } from 'react';
import { useLoopControl } from '../../hooks/useLoopControl';

const ISSUE_TAGS = [
  'Controls',
  'Performance',
  'Visual bug',
  'Audio',
  'Progression',
  'Confusing UX',
];

interface FeedbackPanelProps {
  disabled?: boolean;
  submitting?: boolean;
  onSubmit?: (input: { rating: number | null; text: string; issues: string[] }) => Promise<void>;
  projectPath?: string | null;
}

export const FeedbackPanel: React.FC<FeedbackPanelProps> = ({
  disabled = false,
  submitting = false,
  onSubmit,
  projectPath,
}) => {
  const [rating, setRating] = useState<number | null>(null);
  const [text, setText] = useState('');
  const [issues, setIssues] = useState<string[]>([]);
  const [submitted, setSubmitted] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [submitError, setSubmitError] = useState<string | null>(null);

  const loopControl = useLoopControl(projectPath);

  const toggleIssue = (issue: string) => {
    setIssues(current =>
      current.includes(issue) ? current.filter(value => value !== issue) : [...current, issue],
    );
  };

  const handleSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    setSubmitError(null);
    setIsSubmitting(true);

    try {
      if (onSubmit) {
        await onSubmit({ rating, text, issues });
      } else if (projectPath) {
        await loopControl.submitFeedback(rating ?? 0, text, issues);
      }
      setText('');
      setIssues([]);
      setRating(null);
      setSubmitted(true);
    } catch (err) {
      setSubmitError(err instanceof Error ? err.message : 'Submission failed');
    } finally {
      setIsSubmitting(false);
    }
  };

  const effectiveSubmitting = submitting || isSubmitting;

  return (
    <section className="lux-panel flex flex-col gap-3" aria-label="Player feedback">
      <header className="lux-panel-header">
        <div>
          <h3 className="font-stencil text-[var(--text-title)] m-0">Feedback</h3>
          <p className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] m-0">
            Rate the selected build and report issues.
          </p>
        </div>
      </header>

      <form className="flex flex-col gap-4" onSubmit={handleSubmit}>
        <div>
          <div className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] uppercase tracking-widest mb-2">
            Rating
          </div>
          <div className="flex gap-1" role="radiogroup" aria-label="Star rating">
            {[1, 2, 3, 4, 5].map(value => (
              <button
                key={value}
                type="button"
                className={`text-2xl leading-none transition-colors ${
                  rating !== null && value <= rating ? 'text-yellow-300' : 'text-[var(--color-text-muted)]'
                } disabled:opacity-40`}
                aria-label={`${value} star${value === 1 ? '' : 's'}`}
                aria-checked={rating === value}
                role="radio"
                disabled={disabled || effectiveSubmitting}
                onClick={() => {
                  setRating(value);
                  setSubmitted(false);
                }}
              >
                ★
              </button>
            ))}
          </div>
        </div>

        <div>
          <div className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] uppercase tracking-widest mb-2">
            Issue tags
          </div>
          <div className="flex flex-wrap gap-2">
            {ISSUE_TAGS.map(issue => {
              const selected = issues.includes(issue);
              return (
                <button
                  key={issue}
                  type="button"
                  className={`sys-tag border-[var(--color-line)] ${
                    selected ? 'text-[var(--color-bg)] bg-[var(--color-text)]' : 'text-[var(--color-text-muted)] bg-transparent'
                  } disabled:opacity-40`}
                  disabled={disabled || effectiveSubmitting}
                  onClick={() => {
                    toggleIssue(issue);
                    setSubmitted(false);
                  }}
                >
                  {issue}
                </button>
              );
            })}
          </div>
        </div>

        <label className="flex flex-col gap-2 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] uppercase tracking-widest">
          Notes
          <textarea
            className="min-h-28 resize-y bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] px-3 py-2 rounded-sm font-sans text-sm focus:outline-none focus:border-[var(--color-line-strong)] disabled:opacity-50"
            value={text}
            onChange={event => {
              setText(event.target.value);
              setSubmitted(false);
            }}
            placeholder="What happened during play?"
            disabled={disabled || effectiveSubmitting}
          />
        </label>

        <button
          type="submit"
          className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 disabled:opacity-50 disabled:cursor-not-allowed"
          disabled={disabled || effectiveSubmitting || (rating === null && text.trim().length === 0 && issues.length === 0)}
        >
          {effectiveSubmitting ? 'Submitting' : 'Submit Feedback'}
        </button>
        {submitted && !submitError && (
          <div className="font-terminal text-[var(--text-caption)] text-green-400">Feedback saved.</div>
        )}
        {submitError && (
          <div className="font-terminal text-[var(--text-caption)] text-red-400">{submitError}</div>
        )}
      </form>
    </section>
  );
};
