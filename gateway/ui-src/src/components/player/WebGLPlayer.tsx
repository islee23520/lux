import React, { useEffect, useMemo } from 'react';
import { BuildSelector } from './BuildSelector';
import { EventStream } from './EventStream';
import { FeedbackPanel } from './FeedbackPanel';
import { isPlayableBuild, usePlayer } from '../../hooks/usePlayer';

interface WebGLPlayerProps {
  projectPath?: string;
}

export const WebGLPlayer: React.FC<WebGLPlayerProps> = ({ projectPath }) => {
  const {
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
    refreshBuilds,
    refreshEvents,
    triggerBuild,
    submitFeedback,
  } = usePlayer();

  const iframeSrc = useMemo(() => {
    if (!selectedBuildId || !selectedBuild || !isPlayableBuild(selectedBuild)) {
      return null;
    }
    return `/play/${encodeURIComponent(selectedBuildId)}/`;
  }, [selectedBuild, selectedBuildId]);

  useEffect(() => {
    refreshEvents(projectPath ?? selectedBuild?.project_path, selectedSessionId);
  }, [projectPath, refreshEvents, selectedBuild?.project_path, selectedSessionId]);

  const feedbackDisabled = !selectedSessionId || !(projectPath ?? selectedBuild?.project_path);

  return (
    <section aria-label="WebGL player embed" className="flex h-full min-h-0 flex-col gap-4 p-4">
      <BuildSelector
        builds={builds}
        selectedBuildId={selectedBuildId}
        loading={loading}
        triggeringBuild={triggeringBuild}
        onSelectBuild={setSelectedBuildId}
        onRefresh={refreshBuilds}
        onTriggerBuild={() => {
          triggerBuild(projectPath);
        }}
      />

      {error && (
        <div className="rounded-sm border border-red-500/30 bg-red-500/10 p-3 font-terminal text-[var(--text-caption)] text-red-300">
          {error}
        </div>
      )}

      <div className="grid min-h-0 flex-1 grid-cols-[minmax(0,1fr)_360px] gap-4">
        <main className="lux-panel min-h-0 flex flex-col overflow-hidden" aria-label="Unity WebGL iframe">
          <div className="lux-panel-header">
            <div>
              <h3 className="font-stencil text-[var(--text-title)] m-0">Player</h3>
              <p className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] m-0">
                {selectedBuildId ? selectedBuildId : 'No build selected'}
              </p>
            </div>
            <span className={`sys-tag ${wsConnected ? 'border-green-500/30 text-green-400 bg-green-500/10' : 'border-yellow-500/30 text-yellow-400 bg-yellow-500/10'}`}>
              WS {wsConnected ? 'Live' : 'Offline'}
            </span>
          </div>

          <div className="min-h-0 flex-1 overflow-hidden rounded-sm border border-[var(--color-line)] bg-black">
            {iframeSrc ? (
              <iframe
                title={`Unity WebGL build ${selectedBuildId}`}
                src={iframeSrc}
                className="h-full w-full border-0"
                allow="fullscreen; gamepad; autoplay"
              />
            ) : (
              <div className="flex h-full items-center justify-center p-8 text-center font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
                Select a succeeded WebGL build to load /play/{'{build_id}'}/.
              </div>
            )}
          </div>
        </main>

        <aside className="min-h-0 flex flex-col gap-4" aria-label="Feedback and play events">
          <FeedbackPanel
            disabled={feedbackDisabled}
            submitting={submittingFeedback}
            onSubmit={async input => {
              const resolvedProjectPath = projectPath ?? selectedBuild?.project_path;
              if (!resolvedProjectPath || !selectedSessionId) {
                return;
              }
              await submitFeedback({
                projectPath: resolvedProjectPath,
                sessionId: selectedSessionId,
                rating: input.rating,
                text: input.text,
                issues: input.issues,
              });
            }}
          />
          <EventStream
            events={events}
            loading={eventsLoading}
            connected={wsConnected}
            onRefresh={() => refreshEvents(projectPath ?? selectedBuild?.project_path, selectedSessionId)}
          />
        </aside>
      </div>
    </section>
  );
};
