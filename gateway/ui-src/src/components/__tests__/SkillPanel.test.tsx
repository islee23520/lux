import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { SkillPanel } from '../SkillPanel';

describe('SkillPanel', () => {
  it('renders the built-in skill actions', () => {
    render(<SkillPanel onDispatchSkill={vi.fn()} />);

    expect(screen.getByRole('button', { name: 'compile' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'test' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'screenshot' })).toBeInTheDocument();
  });

  it('dispatches a selected skill', () => {
    const onDispatchSkill = vi.fn();
    render(<SkillPanel onDispatchSkill={onDispatchSkill} />);

    fireEvent.click(screen.getByRole('button', { name: 'compile' }));

    expect(onDispatchSkill).toHaveBeenCalledWith('compile');
  });
});
