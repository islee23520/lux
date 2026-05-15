import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CompilePanel } from '../dashboard/CompilePanel';
import { compileProject } from '../../lib/api';

vi.mock('../../lib/api', () => ({
  compileProject: vi.fn(),
}));

const mockedCompileProject = vi.mocked(compileProject);

describe('CompilePanel', () => {
  beforeEach(() => {
    mockedCompileProject.mockResolvedValue(undefined);
  });

  it('renders the compile button', () => {
    render(<CompilePanel />);

    expect(screen.getByRole('button', { name: 'Compile Project' })).toBeInTheDocument();
    expect(screen.getByText('Ready to compile.')).toBeInTheDocument();
  });

  it('shows compile results after a successful compile', async () => {
    render(<CompilePanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Compile Project' }));

    expect(screen.getByText('Starting compilation...')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByText('Compilation successful!')).toBeInTheDocument());
    expect(screen.getByText('Success')).toBeInTheDocument();
  });

  it('shows compile errors when compilation fails', async () => {
    mockedCompileProject.mockRejectedValue(new Error('Compiler failed'));
    render(<CompilePanel />);

    fireEvent.click(screen.getByRole('button', { name: 'Compile Project' }));

    await waitFor(() => expect(screen.getByText('Error: Compiler failed')).toBeInTheDocument());
    expect(screen.getByText('1 Errors')).toBeInTheDocument();
  });
});
