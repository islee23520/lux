import React from 'react';

interface AmbiguityBarProps {
  score: number | null | undefined;
  label?: string;
}

function clampScore(score: number): number {
  if (!Number.isFinite(score)) {
    return 0;
  }
  return Math.min(1, Math.max(0, score));
}

function getTone(score: number): { label: string; className: string; color: string } {
  if (score < 0.3) {
    return { label: 'Low', className: 'text-green-400', color: 'var(--green, #34d399)' };
  }
  if (score <= 0.7) {
    return { label: 'Medium', className: 'text-yellow-400', color: 'var(--yellow, #facc15)' };
  }
  return { label: 'High', className: 'text-red-400', color: 'var(--red, #fb7185)' };
}

export const AmbiguityBar: React.FC<AmbiguityBarProps> = ({ score, label = 'Ambiguity' }) => {
  const value = clampScore(score ?? 0);
  const tone = getTone(value);
  const percent = Math.round(value * 100);

  return (
    <div className="flex flex-col gap-2" aria-label={`${label} score ${percent}%`}>
      <div className="flex items-center justify-between font-terminal text-[var(--text-caption)]">
        <span className="uppercase tracking-widest text-[var(--color-text-muted)]">{label}</span>
        <span className={tone.className}>{tone.label} · {percent}%</span>
      </div>
      <div className="h-2 rounded-full bg-[var(--color-surface-raised)] border border-[var(--color-line)] overflow-hidden">
        <div
          className="h-full transition-[width] duration-300"
          style={{ width: `${percent}%`, backgroundColor: tone.color }}
        />
      </div>
    </div>
  );
};
