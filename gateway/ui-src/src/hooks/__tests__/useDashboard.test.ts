import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useDashboard } from '../useDashboard';
import { detectProject } from '../../lib/api';

vi.mock('../../lib/api', () => ({
  detectProject: vi.fn(),
}));

const mockedDetectProject = vi.mocked(detectProject);

describe('useDashboard', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    mockedDetectProject.mockResolvedValue({ name: 'Neon Glitch', path: '/unity/project', unityVersion: '6000.0' });
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: true }));
  });

  it('exposes the initial dashboard state', () => {
    const { result } = renderHook(() => useDashboard());

    expect(result.current.activePanel).toBe('overview');
    expect(result.current.sidebarCollapsed).toBe(false);
    expect(result.current.projectInfo).toBeNull();
    expect(result.current.serverStatus).toBe('disconnected');
    expect(result.current.loading).toBe(true);
  });

  it('polls the health endpoint', async () => {
    renderHook(() => useDashboard());

    await act(async () => {
      await Promise.resolve();
    });

    expect(fetch).toHaveBeenCalledWith('/api/health');

    await act(async () => {
      vi.advanceTimersByTime(10_000);
      await Promise.resolve();
    });

    expect(fetch).toHaveBeenCalledTimes(2);
  });

  it('updates server status from health checks', async () => {
    vi.stubGlobal('fetch', vi.fn().mockResolvedValue({ ok: false }));
    const { result } = renderHook(() => useDashboard());

    await act(async () => {
      await Promise.resolve();
    });

    expect(result.current.serverStatus).toBe('disconnected');

    vi.mocked(fetch).mockResolvedValue({ ok: true } as Response);

    await act(async () => {
      vi.advanceTimersByTime(10_000);
      await Promise.resolve();
    });

    expect(result.current.serverStatus).toBe('connected');
  });
});
