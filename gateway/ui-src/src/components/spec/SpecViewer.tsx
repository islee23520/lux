import React, { useMemo, useState } from 'react';
import { useSpec } from '../../hooks/useSpec';
import type { DomainSpec, SpecDomainKey, SpecDomains } from '../../hooks/useSpec';
import { AmbiguityBar } from './AmbiguityBar';
import { DomainTab } from './DomainTab';

interface SpecViewerProps {
  projectPath: string | null;
}

interface DomainDefinition {
  key: SpecDomainKey;
  label: string;
  field: keyof Pick<SpecDomains, 'design' | 'architecture' | 'art_style' | 'audio' | 'narrative' | 'levels' | 'ui_ux'>;
}

const DOMAINS: DomainDefinition[] = [
  { key: 'design', label: 'Design', field: 'design' },
  { key: 'architecture', label: 'Architecture', field: 'architecture' },
  { key: 'art-style', label: 'Art Style', field: 'art_style' },
  { key: 'audio', label: 'Audio', field: 'audio' },
  { key: 'narrative', label: 'Narrative', field: 'narrative' },
  { key: 'levels', label: 'Levels', field: 'levels' },
  { key: 'ui-ux', label: 'UI/UX', field: 'ui_ux' },
];

function getDomainSpec(domains: SpecDomains | undefined, field: DomainDefinition['field']): DomainSpec | null {
  return domains?.[field] ?? null;
}

export const SpecViewer: React.FC<SpecViewerProps> = ({ projectPath }) => {
  const [activeDomain, setActiveDomain] = useState<SpecDomainKey>('design');
  const { spec, ambiguity, loading, saving, wsConnected, error, refresh, loadDomain, saveDomain } = useSpec(projectPath);

  const selectedDomain = useMemo(
    () => DOMAINS.find(domain => domain.key === activeDomain) ?? DOMAINS[0],
    [activeDomain],
  );
  const selectedSpec = getDomainSpec(spec?.domains, selectedDomain.field);

  const domainStatuses = DOMAINS.map(domain => {
    const domainSpec = getDomainSpec(spec?.domains, domain.field);
    const score = ambiguity?.domains[domain.key] ?? domainSpec?.ambiguity_score ?? 0;
    return { ...domain, domainSpec, score };
  });

  return (
    <div className="flex flex-col gap-4">
      <section aria-label="Workbench Header" className="lux-panel">
        <header className="lux-panel-header">
          <div>
            <h1 className="font-stencil text-[var(--text-title)] m-0">Workbench</h1>
            <p className="m-0 mt-1 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
              {spec ? `${spec.project_name || 'Lux Project'} · ${spec.status} · v${spec.version}` : 'Define specs and control AI from one surface'}
            </p>
          </div>
          <div className="flex items-center gap-3">
            <div className="connection-status">
              <span className={`status-dot ${wsConnected ? 'connected' : 'disconnected'}`} />
              <span className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                {wsConnected ? 'spec:update live' : 'spec:update offline'}
              </span>
            </div>
            <button
              type="button"
              className="px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              onClick={() => void refresh()}
              disabled={loading || !projectPath}
            >
              {loading ? 'Refreshing...' : 'Refresh'}
            </button>
          </div>
        </header>
        <div className="lux-panel-body">
          <AmbiguityBar score={ambiguity?.overall ?? spec?.overall_ambiguity ?? null} label="Overall Ambiguity" />
          {error && <div className="mt-3 text-red-400 font-terminal text-[var(--text-caption)]">{error}</div>}
        </div>
      </section>

      <section className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3" aria-label="Spec markdown status">
        {domainStatuses.map(domain => (
          <button
            key={domain.key}
            type="button"
            className={`text-left rounded-md border p-3 bg-[var(--color-surface)] transition-colors ${domain.key === activeDomain ? 'border-[var(--blue,#38bdf8)]' : 'border-[var(--color-line)] hover:border-[var(--color-line-strong)]'}`}
            onClick={() => setActiveDomain(domain.key)}
          >
            <div className="flex items-center justify-between gap-2">
              <span className="font-terminal text-[var(--text-caption)] uppercase tracking-widest text-[var(--color-text)]">{domain.label}</span>
              <span className={`sys-tag ${domain.domainSpec?.defined ? 'border-green-500/30 text-green-400 bg-green-500/10' : 'border-yellow-500/30 text-yellow-400 bg-yellow-500/10'}`}>
                {domain.domainSpec?.defined ? 'Defined' : 'Draft'}
              </span>
            </div>
            <div className="mt-2 font-terminal text-[var(--text-micro)] text-[var(--color-text-muted)] truncate">
              {domain.domainSpec?.content_path ?? `${domain.key}.md`}
            </div>
            <div className="mt-3 h-1.5 rounded-full bg-[var(--color-surface-raised)] overflow-hidden">
              <div className="h-full bg-[var(--blue,#38bdf8)]" style={{ width: `${Math.round(domain.score * 100)}%` }} />
            </div>
          </button>
        ))}
      </section>

      <section className="lux-panel flex flex-col" aria-label="Spec Domains">
        <div className="flex flex-wrap gap-2 p-3 border-b border-[var(--color-line)] bg-[var(--color-surface-raised)]">
          {DOMAINS.map(domain => {
            const domainSpec = getDomainSpec(spec?.domains, domain.field);
            const score = ambiguity?.domains[domain.key] ?? domainSpec?.ambiguity_score ?? 0;
            const active = domain.key === activeDomain;
            return (
              <button
                key={domain.key}
                type="button"
                className={`px-3 py-2 rounded-sm border font-terminal text-[var(--text-caption)] uppercase tracking-widest transition-colors ${
                  active
                    ? 'border-[var(--blue,#38bdf8)] text-[var(--blue,#38bdf8)] bg-sky-500/10'
                    : 'border-[var(--color-line)] text-[var(--color-text-muted)] hover:text-[var(--color-text)] hover:border-[var(--color-line-strong)]'
                }`}
                onClick={() => setActiveDomain(domain.key)}
              >
                {domain.label} <span className="opacity-70">{Math.round(score * 100)}%</span>
              </button>
            );
          })}
        </div>

        <div className="p-4">
          {!projectPath ? (
            <div className="panel-card">Project path is required to load spec data.</div>
          ) : loading && !spec ? (
            <div className="panel-card">Loading spec...</div>
          ) : (
            <DomainTab
              key={selectedDomain.key}
              domainKey={selectedDomain.key}
              label={selectedDomain.label}
              projectPath={projectPath}
              domainSpec={selectedSpec}
              ambiguityScore={ambiguity?.domains[selectedDomain.key] ?? null}
              schellEvaluation={spec?.schell_evaluation ?? null}
              saving={saving}
              loadDomain={loadDomain}
              saveDomain={saveDomain}
            />
          )}
        </div>
      </section>
    </div>
  );
};
