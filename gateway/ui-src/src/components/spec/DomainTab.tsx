import React, { useEffect, useMemo, useState } from 'react';
import type { DomainSpec, SchellEvaluation, SpecDomainKey } from '../../hooks/useSpec';
import { AmbiguityBar } from './AmbiguityBar';
import { SchellPhases } from './SchellPhases';

interface DomainTabProps {
  domainKey: SpecDomainKey;
  label: string;
  projectPath: string | null;
  domainSpec: DomainSpec | null;
  ambiguityScore: number | null;
  schellEvaluation: SchellEvaluation | null;
  saving: boolean;
  loadDomain: (domain: SpecDomainKey) => Promise<{ content: string }>;
  saveDomain: (domain: SpecDomainKey, content: string) => Promise<void>;
}

type AiStatus = 'idle' | 'starting' | 'queued';

function renderMarkdownLine(line: string, index: number): React.ReactNode {
  if (line.trim().length === 0) {
    return <div key={index} className="h-4" />;
  }

  const heading = /^(#{1,3})\s+(.+)$/.exec(line);
  if (heading) {
    const sizeClass = heading[1].length === 1 ? 'text-xl' : heading[1].length === 2 ? 'text-lg' : 'text-base';
    return <div key={index} className={`${sizeClass} font-bold text-[var(--color-text)] mt-3`}>{heading[2]}</div>;
  }

  const bullet = /^[-*]\s+(.+)$/.exec(line);
  if (bullet) {
    return <li key={index} className="ml-5 text-[var(--color-text-muted)]">{bullet[1]}</li>;
  }

  const quote = /^>\s+(.+)$/.exec(line);
  if (quote) {
    return <blockquote key={index} className="border-l-2 border-[var(--blue,#38bdf8)] pl-3 text-[var(--color-text-muted)] italic">{quote[1]}</blockquote>;
  }

  return <p key={index} className="my-1 text-[var(--color-text-muted)]">{line}</p>;
}

export const DomainTab: React.FC<DomainTabProps> = ({
  domainKey,
  label,
  projectPath,
  domainSpec,
  ambiguityScore,
  schellEvaluation,
  saving,
  loadDomain,
  saveDomain,
}) => {
  const [content, setContent] = useState('');
  const [initialContent, setInitialContent] = useState('');
  const [loadingContent, setLoadingContent] = useState(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [aiStatus, setAiStatus] = useState<AiStatus>('idle');
  const [aiPrompt, setAiPrompt] = useState('');
  const [aiMessages, setAiMessages] = useState<string[]>([
    'AI control is ready. Ask it to clarify a mechanic, rewrite a section, or generate missing questions.',
  ]);

  useEffect(() => {
    if (!projectPath) {
      setContent('');
      setInitialContent('');
      return;
    }

    let cancelled = false;
    setLoadingContent(true);
    setLocalError(null);
    loadDomain(domainKey)
      .then(response => {
        if (!cancelled) {
          setContent(response.content);
          setInitialContent(response.content);
        }
      })
      .catch(err => {
        if (!cancelled) {
          setLocalError(err instanceof Error ? err.message : String(err));
        }
      })
      .finally(() => {
        if (!cancelled) {
          setLoadingContent(false);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [domainKey, loadDomain, projectPath]);

  const dirty = content !== initialContent;
  const preview = useMemo(() => content.split('\n').map(renderMarkdownLine), [content]);

  const handleSave = async () => {
    setLocalError(null);
    try {
      await saveDomain(domainKey, content);
      setInitialContent(content);
    } catch (err) {
      setLocalError(err instanceof Error ? err.message : String(err));
    }
  };

  const handleAiSession = () => {
    setAiStatus('starting');
    window.setTimeout(() => setAiStatus('queued'), 500);
  };

  const sendAiPrompt = () => {
    const prompt = aiPrompt.trim();
    if (!prompt) return;
    setAiStatus('queued');
    setAiMessages(prev => [
      ...prev,
      `You: ${prompt}`,
      `Lux AI: Review ${label}.md with the current draft, ambiguity score, and Schell phase status before applying edits.`,
    ]);
    setAiPrompt('');
  };

  return (
    <div className="grid grid-cols-[minmax(0,1fr)_360px] gap-4 h-full min-h-0">
      <section className="lux-panel min-h-0 flex flex-col" aria-label={`${label} Markdown Editor`}>
        <header className="lux-panel-header">
          <div>
            <h2 className="font-stencil text-[var(--text-title)] m-0">{label}</h2>
            {domainSpec && (
              <p className="m-0 mt-1 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                {domainSpec.defined ? 'Defined' : 'Draft'} · {domainSpec.content_path}
              </p>
            )}
          </div>
          <div className="flex items-center gap-3">
            {dirty && <span className="sys-tag border-yellow-500/30 text-yellow-400 bg-yellow-500/10">Unsaved</span>}
            <button
              type="button"
              className="px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              onClick={handleAiSession}
              disabled={!projectPath || aiStatus === 'starting'}
            >
              {aiStatus === 'idle' ? 'Start AI Session' : aiStatus === 'starting' ? 'Starting...' : 'AI Session Queued'}
            </button>
            <button
              type="button"
              className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              onClick={handleSave}
              disabled={!dirty || saving || loadingContent || !projectPath}
            >
              {saving ? 'Saving...' : 'Save'}
            </button>
          </div>
        </header>

        <div className="lux-panel-body grid grid-cols-2 gap-4 flex-1 min-h-0">
          <div className="flex flex-col min-h-0">
            <label className="font-terminal text-[var(--text-caption)] uppercase tracking-widest text-[var(--color-text-muted)] mb-2" htmlFor={`spec-editor-${domainKey}`}>
              Markdown
            </label>
            <textarea
              id={`spec-editor-${domainKey}`}
              className="flex-1 min-h-[420px] resize-none rounded-md border border-[var(--color-line)] bg-black/70 p-3 font-mono text-sm text-[var(--color-text)] outline-none focus:border-[var(--blue,#38bdf8)]"
              value={content}
              onChange={event => setContent(event.target.value)}
              disabled={loadingContent || !projectPath}
              spellCheck={false}
            />
          </div>
          <div className="flex flex-col min-h-0">
            <div className="font-terminal text-[var(--text-caption)] uppercase tracking-widest text-[var(--color-text-muted)] mb-2">Preview</div>
            <div className="flex-1 min-h-[420px] overflow-y-auto rounded-md border border-[var(--color-line)] bg-[var(--color-surface-raised)] p-3">
              {loadingContent ? (
                <p className="text-[var(--color-text-muted)]">Loading domain content...</p>
              ) : content.trim().length > 0 ? (
                preview
              ) : (
                <p className="text-[var(--color-text-muted)] italic">No markdown content.</p>
              )}
            </div>
          </div>
          {localError && <div className="col-span-2 text-red-400 font-terminal text-[var(--text-caption)]">{localError}</div>}
        </div>
      </section>

      <aside className="flex flex-col gap-4 min-h-0">
        <section className="panel-card m-0" aria-label={`${label} Ambiguity`}>
          <AmbiguityBar score={ambiguityScore ?? domainSpec?.ambiguity_score ?? null} />
          {domainSpec?.last_evaluated && (
            <p className="mt-3 mb-0 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
              Last evaluated {new Date(domainSpec.last_evaluated).toLocaleString()}
            </p>
          )}
        </section>
        <section className="panel-card m-0 flex flex-col min-h-[260px]" aria-label="AI Control">
          <h3 className="panel-title">AI Control</h3>
          <div className="connection-status">
            <span className={`status-dot ${aiStatus === 'idle' ? 'disconnected' : 'connected'}`} />
            <span className="text-[var(--color-text-muted)]">
              {aiStatus === 'idle' ? 'Idle' : aiStatus === 'starting' ? 'Starting' : 'Queued'}
            </span>
          </div>
          <div className="mt-4 flex-1 min-h-0 overflow-y-auto rounded-md border border-[var(--color-line)] bg-black/40 p-3 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
            {aiMessages.map((message, index) => (
              <p key={`${message}-${index}`} className="mb-2 last:mb-0 whitespace-pre-wrap">{message}</p>
            ))}
          </div>
          <div className="mt-3 flex gap-2">
            <input
              type="text"
              className="min-w-0 flex-1 rounded-sm border border-[var(--color-line)] bg-black/70 px-3 py-2 font-terminal text-[var(--text-caption)] text-[var(--color-text)] outline-none focus:border-[var(--blue,#38bdf8)]"
              placeholder={`Ask AI about ${label}.md`}
              value={aiPrompt}
              onChange={event => setAiPrompt(event.target.value)}
              onKeyDown={event => {
                if (event.key === 'Enter') sendAiPrompt();
              }}
            />
            <button
              type="button"
              className="px-3 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm disabled:opacity-50"
              disabled={!aiPrompt.trim()}
              onClick={sendAiPrompt}
            >
              Send
            </button>
          </div>
        </section>
        <SchellPhases evaluation={schellEvaluation} />
      </aside>
    </div>
  );
};
