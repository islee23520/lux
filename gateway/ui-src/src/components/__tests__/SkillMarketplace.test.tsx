import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { listSkills } from '../../lib/api';
import { SkillMarketplace } from '../dashboard/SkillMarketplace';

vi.mock('../../lib/api', () => ({
  listSkills: vi.fn(),
}));

describe('SkillMarketplace', () => {
  beforeEach(() => {
    vi.mocked(listSkills).mockReset();
  });

  it('does not read skills until explicitly requested', () => {
    render(<SkillMarketplace />);

    expect(listSkills).not.toHaveBeenCalled();
    expect(screen.getByText('Skills have not been loaded yet.')).toBeInTheDocument();
  });

  it('loads skills passively on button click', async () => {
    vi.mocked(listSkills).mockResolvedValue([
      {
        name: 'lux-unity',
        version: '1.0.0',
        description: 'Unity skill',
        scope: 'core',
        directory_path: '/repo/Skills/lux-unity',
        manifest: {},
      },
    ]);

    render(<SkillMarketplace />);
    fireEvent.click(screen.getByRole('button', { name: 'Load skills' }));

    await waitFor(() => expect(screen.getByText('lux-unity')).toBeInTheDocument());
    expect(listSkills).toHaveBeenCalledTimes(1);
  });

  it('reports load errors without fallback data', async () => {
    vi.mocked(listSkills).mockRejectedValue(new Error('offline'));

    render(<SkillMarketplace />);
    fireEvent.click(screen.getByRole('button', { name: 'Load skills' }));

    await waitFor(() => expect(screen.getByRole('alert')).toHaveTextContent('offline'));
    expect(screen.queryByText('UI Builder')).not.toBeInTheDocument();
  });
});
