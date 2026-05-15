import { act, renderHook } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useCapture } from '../useCapture';

const mockFetch = vi.fn();

const createResponse = (data: unknown, ok = true, statusText = 'OK') => ({
  ok,
  statusText,
  json: async () => data,
});

const flushPromises = async () => {
  await Promise.resolve();
  await Promise.resolve();
};

describe('useCapture', () => {
  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    mockFetch.mockReset();
  });

  it('starts in an idle state', () => {
    vi.stubGlobal('fetch', mockFetch);
    const { result } = renderHook(() => useCapture());

    expect(result.current.status).toBe('idle');
    expect(result.current.sessionId).toBeNull();
    expect(result.current.error).toBeNull();
    expect(result.current.sessionInfo).toBeNull();
  });

  it('starts capture and polls until the session becomes streaming', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('fetch', mockFetch);
    mockFetch
      .mockResolvedValueOnce(createResponse({
        session_id: 'session-1',
        status: 'starting',
        width: 1280,
        height: 720,
      }))
      .mockResolvedValueOnce(createResponse({
        session_id: 'session-1',
        status: 'streaming',
        width: 1280,
        height: 720,
      }));

    const { result } = renderHook(() => useCapture());

    await act(async () => {
      await result.current.startCapture('/project', 1280, 720, 30);
    });

    expect(result.current.status).toBe('starting');
    expect(result.current.sessionId).toBe('session-1');
    expect(mockFetch).toHaveBeenNthCalledWith(1, '/api/unity/capture/sessions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        project_path: '/project',
        width: 1280,
        height: 720,
        fps: 30,
      }),
    });

    await act(async () => {
      vi.advanceTimersByTime(2000);
      await flushPromises();
    });

    expect(mockFetch).toHaveBeenNthCalledWith(2, '/api/unity/capture/sessions/session-1');
    expect(result.current.status).toBe('streaming');
    expect(result.current.sessionInfo).toEqual({
      session_id: 'session-1',
      status: 'streaming',
      width: 1280,
      height: 720,
    });
  });

  it('clears the active session after stopCapture succeeds', async () => {
    vi.stubGlobal('fetch', mockFetch);
    mockFetch
      .mockResolvedValueOnce(createResponse({
        session_id: 'session-1',
        status: 'streaming',
      }))
      .mockResolvedValueOnce(createResponse(undefined));

    const { result } = renderHook(() => useCapture());

    await act(async () => {
      await result.current.startCapture('/project', 1280, 720, 30);
    });

    await act(async () => {
      await result.current.stopCapture();
    });

    expect(result.current.status).toBe('idle');
    expect(result.current.sessionId).toBeNull();
    expect(result.current.sessionInfo).toBeNull();
    expect(mockFetch).toHaveBeenNthCalledWith(2, '/api/unity/capture/sessions/session-1', { method: 'DELETE' });
  });

  it('reports start capture errors', async () => {
    vi.stubGlobal('fetch', mockFetch);
    mockFetch.mockResolvedValueOnce({ ok: false, statusText: 'Bad Gateway' });

    const { result } = renderHook(() => useCapture());

    await act(async () => {
      const sessionId = await result.current.startCapture('/project', 640, 480, 15);
      expect(sessionId).toBeNull();
    });

    expect(result.current.status).toBe('error');
    expect(result.current.error).toBe('Failed to start capture: Bad Gateway');
  });

  it('reports polling errors while streaming', async () => {
    vi.useFakeTimers();
    vi.stubGlobal('fetch', mockFetch);
    mockFetch
      .mockResolvedValueOnce(createResponse({
        session_id: 'session-1',
        status: 'starting',
      }))
      .mockResolvedValueOnce({ ok: false, statusText: 'Gone' });

    const { result } = renderHook(() => useCapture());

    await act(async () => {
      await result.current.startCapture('/project', 800, 600, 24);
    });

    await act(async () => {
      vi.advanceTimersByTime(2000);
      await flushPromises();
    });

    expect(result.current.status).toBe('error');
    expect(result.current.error).toBe('Failed to get status: Gone');
  });

  it('reports stop capture errors', async () => {
    vi.stubGlobal('fetch', mockFetch);
    mockFetch
      .mockResolvedValueOnce(createResponse({
        session_id: 'session-1',
        status: 'streaming',
      }))
      .mockResolvedValueOnce({ ok: false, statusText: 'Conflict' });

    const { result } = renderHook(() => useCapture());

    await act(async () => {
      await result.current.startCapture('/project', 1024, 768, 30);
    });

    await act(async () => {
      await result.current.stopCapture();
    });

    expect(result.current.status).toBe('error');
    expect(result.current.error).toBe('Failed to stop capture: Conflict');
    expect(result.current.sessionId).toBe('session-1');
  });
});
