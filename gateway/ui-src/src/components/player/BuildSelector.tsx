import React from 'react';
import type { BuildJob } from '../../hooks/usePlayer';
import { buildStatusLabel, isPlayableBuild } from '../../hooks/usePlayer';

interface BuildSelectorProps {
  builds: BuildJob[];
  selectedBuildId: string | null;
  loading: boolean;
  triggeringBuild: boolean;
  onSelectBuild: (buildId: string) => void;
  onRefresh: () => void;
  onTriggerBuild: () => void;
}

function formatDate(value: string | null): string {
  if (!value) {
    return 'No timestamp';
  }
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return date.toLocaleString();
}

function optionLabel(job: BuildJob): string {
  const status = buildStatusLabel(job.status);
  const date = formatDate(job.completed_at ?? job.started_at);
  return `${job.build_id.slice(0, 8)} · ${status} · ${date}`;
}

export const BuildSelector: React.FC<BuildSelectorProps> = ({
  builds,
  selectedBuildId,
  loading,
  triggeringBuild,
  onSelectBuild,
  onRefresh,
  onTriggerBuild,
}) => {
  const selectedBuild = builds.find(build => build.build_id === selectedBuildId) ?? null;

  return (
    <section className="lux-panel flex flex-col gap-3" aria-label="WebGL build selector">
      <header className="lux-panel-header">
        <div>
          <h2 className="font-stencil text-[var(--text-title)] m-0">WebGL Player</h2>
          <p className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] m-0">
            Select a Unity WebGL build to embed.
          </p>
        </div>
        <div className="flex gap-2 items-center">
          <button
            type="button"
            className="px-3 py-2 bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:border-[var(--color-line-strong)] disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={onRefresh}
            disabled={loading}
          >
            {loading ? 'Loading' : 'Refresh'}
          </button>
          <button
            type="button"
            className="px-3 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 disabled:opacity-50 disabled:cursor-not-allowed"
            onClick={onTriggerBuild}
            disabled={triggeringBuild}
          >
            {triggeringBuild ? 'Building' : 'Build'}
          </button>
        </div>
      </header>

      <label className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] uppercase tracking-widest" htmlFor="webgl-build-select">
        Build
      </label>
      <select
        id="webgl-build-select"
        className="bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] px-3 py-2 rounded-sm font-terminal text-[var(--text-caption)] focus:outline-none focus:border-[var(--color-line-strong)]"
        value={selectedBuildId ?? ''}
        onChange={event => onSelectBuild(event.target.value)}
        disabled={loading || builds.length === 0}
      >
        {builds.length === 0 ? (
          <option value="">No builds available</option>
        ) : (
          builds.map(build => (
            <option key={build.build_id} value={build.build_id}>
              {optionLabel(build)}
            </option>
          ))
        )}
      </select>

      {selectedBuild && (
        <div className="grid grid-cols-2 gap-3 text-[var(--text-caption)] font-terminal">
          <div className="rounded-sm border border-[var(--color-line)] bg-[var(--color-surface-raised)] p-3">
            <div className="text-[var(--color-text-muted)] uppercase tracking-widest">Status</div>
            <div className={isPlayableBuild(selectedBuild) ? 'text-green-400' : 'text-yellow-400'}>
              {buildStatusLabel(selectedBuild.status)}
            </div>
          </div>
          <div className="rounded-sm border border-[var(--color-line)] bg-[var(--color-surface-raised)] p-3">
            <div className="text-[var(--color-text-muted)] uppercase tracking-widest">Progress</div>
            <div>{Math.round(selectedBuild.progress * 100)}%</div>
          </div>
        </div>
      )}
    </section>
  );
};
