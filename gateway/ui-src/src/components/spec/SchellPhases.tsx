import React from 'react';
import type { PillarStatus, SchellEvaluation } from '../../hooks/useSpec';

interface SchellPhasesProps {
  evaluation: SchellEvaluation | null;
}

interface PhaseViewModel {
  name: string;
  status: PillarStatus;
  score: number;
  summary: string | null;
}

function statusLabel(status: PillarStatus): 'Missing' | 'Partial' | 'Complete' {
  if (status === 'Strong') {
    return 'Complete';
  }
  if (status === 'NeedsWork') {
    return 'Partial';
  }
  return 'Missing';
}

function statusClass(status: PillarStatus): string {
  if (status === 'Strong') {
    return 'border-green-500/30 text-green-400 bg-green-500/10';
  }
  if (status === 'NeedsWork') {
    return 'border-yellow-500/30 text-yellow-400 bg-yellow-500/10';
  }
  return 'border-red-500/30 text-red-400 bg-red-500/10';
}

interface StatusScore {
  status: PillarStatus;
  score: number;
}

function averageScores(items: StatusScore[]): number {
  if (items.length === 0) {
    return 0;
  }
  return items.reduce((sum, item) => sum + item.score, 0) / items.length;
}

function combinedStatus(items: StatusScore[]): PillarStatus {
  if (items.every(item => item.status === 'Strong')) {
    return 'Strong';
  }
  if (items.some(item => item.status !== 'Missing')) {
    return 'NeedsWork';
  }
  return 'Missing';
}

function toPhaseModels(evaluation: SchellEvaluation): PhaseViewModel[] {
  const themePillars = [evaluation.phase2_tetrad.aesthetics, evaluation.phase4_motivation];
  return [
    {
      name: 'Experience',
      status: evaluation.phase1_experience.status,
      score: evaluation.phase1_experience.score,
      summary: evaluation.phase1_experience.summary,
    },
    {
      name: 'Theme',
      status: combinedStatus(themePillars),
      score: averageScores(themePillars),
      summary: evaluation.phase4_motivation.summary ?? evaluation.phase2_tetrad.aesthetics.description,
    },
    {
      name: 'Mechanics',
      status: evaluation.phase2_tetrad.mechanics.status,
      score: evaluation.phase2_tetrad.mechanics.score,
      summary: evaluation.phase2_tetrad.mechanics.description,
    },
    {
      name: 'Technology',
      status: evaluation.phase2_tetrad.technology.status,
      score: evaluation.phase2_tetrad.technology.score,
      summary: evaluation.phase2_tetrad.technology.description,
    },
    {
      name: 'Story',
      status: evaluation.phase2_tetrad.story.status,
      score: evaluation.phase2_tetrad.story.score,
      summary: evaluation.phase2_tetrad.story.description,
    },
  ];
}

export const SchellPhases: React.FC<SchellPhasesProps> = ({ evaluation }) => {
  if (!evaluation) {
    return (
      <section className="panel-card m-0" aria-label="Schell 5-Phase Evaluation">
        <h3 className="panel-title">Schell 5-Phase</h3>
        <p className="text-[var(--color-text-muted)] m-0">No Schell evaluation loaded.</p>
      </section>
    );
  }

  return (
    <section className="panel-card m-0" aria-label="Schell 5-Phase Evaluation">
      <h3 className="panel-title">Schell 5-Phase</h3>
      <div className="grid gap-3">
        {toPhaseModels(evaluation).map(phase => (
          <article key={phase.name} className="rounded-md border border-[var(--color-line)] bg-[var(--color-surface-raised)] p-3">
            <div className="flex items-center justify-between gap-3">
              <div className="font-bold text-[var(--color-text)]">{phase.name}</div>
              <span className={`sys-tag ${statusClass(phase.status)}`}>{statusLabel(phase.status)}</span>
            </div>
            <div className="mt-2 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
              Score {Math.round(Math.min(1, Math.max(0, phase.score)) * 100)}%
            </div>
            {phase.summary && <p className="mt-2 mb-0 text-[var(--color-text-muted)]">{phase.summary}</p>}
          </article>
        ))}
      </div>
    </section>
  );
};
