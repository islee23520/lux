import React, { useEffect, useMemo, useRef, useState } from 'react';
import type { PlayEvent, PlayEventType } from '../../hooks/usePlayer';

interface EventStreamProps {
  events: PlayEvent[];
  loading: boolean;
  connected: boolean;
  onRefresh: () => void;
}

function eventTypeLabel(eventType: PlayEventType): string {
  return typeof eventType === 'string' ? eventType : eventType.Custom;
}

function formatTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleTimeString();
}

function summarizePayload(payload: unknown): string {
  if (payload === null || payload === undefined) {
    return 'No payload';
  }
  if (typeof payload === 'string') {
    return payload;
  }
  if (typeof payload === 'number' || typeof payload === 'boolean') {
    return String(payload);
  }
  try {
    return JSON.stringify(payload);
  } catch {
    return 'Unserializable payload';
  }
}

export const EventStream: React.FC<EventStreamProps> = ({ events, loading, connected, onRefresh }) => {
  const [autoScroll, setAutoScroll] = useState(true);
  const endRef = useRef<HTMLDivElement>(null);
  const sortedEvents = useMemo(
    () => [...events].sort((a, b) => a.sequence - b.sequence || a.timestamp.localeCompare(b.timestamp)),
    [events],
  );

  useEffect(() => {
    if (autoScroll) {
      endRef.current?.scrollIntoView({ behavior: 'smooth' });
    }
  }, [sortedEvents, autoScroll]);

  return (
    <section className="lux-panel min-h-0 flex flex-1 flex-col" aria-label="Real-time play event stream">
      <header className="lux-panel-header">
        <div>
          <h3 className="font-stencil text-[var(--text-title)] m-0">Play Events</h3>
          <p className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] m-0">
            {connected ? 'Live stream connected' : 'Live stream disconnected'}
          </p>
        </div>
        <div className="flex gap-2 items-center">
          <button
            type="button"
            className="sys-tag border-[var(--color-line)] text-[var(--color-text-muted)] bg-transparent"
            onClick={() => setAutoScroll(current => !current)}
          >
            Auto {autoScroll ? 'On' : 'Off'}
          </button>
          <button
            type="button"
            className="px-3 py-2 bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:border-[var(--color-line-strong)] disabled:opacity-50"
            onClick={onRefresh}
            disabled={loading}
          >
            {loading ? 'Loading' : 'Refresh'}
          </button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto rounded-sm border border-[var(--color-line)] bg-[var(--color-surface-raised)]">
        {sortedEvents.length === 0 ? (
          <div className="p-4 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
            No play events yet.
          </div>
        ) : (
          <ol className="m-0 list-none p-0">
            {sortedEvents.map(event => (
              <li
                key={`${event.session_id}-${event.sequence}-${event.timestamp}`}
                className="border-b border-[var(--color-line)] p-3 last:border-b-0"
              >
                <div className="flex items-center justify-between gap-3">
                  <span className="font-terminal text-[var(--text-caption)] text-[var(--color-text)] uppercase tracking-wider">
                    {eventTypeLabel(event.event_type)}
                  </span>
                  <span className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                    #{event.sequence} · {formatTime(event.timestamp)}
                  </span>
                </div>
                <div className="mt-2 break-words font-mono text-xs text-[var(--color-text-muted)]">
                  {summarizePayload(event.payload)}
                </div>
              </li>
            ))}
          </ol>
        )}
        <div ref={endRef} />
      </div>
    </section>
  );
};
