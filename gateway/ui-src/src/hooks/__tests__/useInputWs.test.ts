import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useInputWs } from '../useInputWs';

class MockWebSocket {
  static instances: MockWebSocket[] = [];
  static readonly OPEN = 1;
  static readonly CONNECTING = 0;
  static readonly CLOSING = 2;
  static readonly CLOSED = 3;

  readonly url: string;
  readyState = MockWebSocket.CONNECTING;
  onopen: (() => void) | null = null;
  onclose: (() => void) | null = null;
  onerror: (() => void) | null = null;
  send = vi.fn();
  close = vi.fn(() => {
    this.readyState = 3;
  });

  constructor(url: string) {
    this.url = url;
    MockWebSocket.instances.push(this);
  }
}

describe('useInputWs', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    MockWebSocket.instances = [];
    vi.stubGlobal('WebSocket', MockWebSocket);
  });

  afterEach(() => {
    vi.clearAllTimers();
    vi.useRealTimers();
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('stays disconnected without a session id', () => {
    const { result } = renderHook(() => useInputWs(null));

    expect(result.current.isConnected).toBe(false);
    expect(MockWebSocket.instances).toHaveLength(0);
  });

  it('connects to the session input websocket', async () => {
    const { result } = renderHook(() => useInputWs('session-1'));
    const ws = MockWebSocket.instances[0];

    expect(ws.url).toContain('/api/unity/capture/sessions/session-1/input');

    act(() => {
      ws.readyState = MockWebSocket.OPEN;
      ws.onopen?.();
    });

    expect(result.current.isConnected).toBe(true);
  });

  it('disconnects when the session id is cleared', () => {
    const { result, rerender } = renderHook(
      ({ sessionId }: { sessionId: string | null }) => useInputWs(sessionId),
      { initialProps: { sessionId: 'session-1' as string | null } },
    );
    const ws = MockWebSocket.instances[0];

    act(() => {
      ws.readyState = MockWebSocket.OPEN;
      ws.onopen?.();
    });

    expect(result.current.isConnected).toBe(true);

    act(() => {
      rerender({ sessionId: null });
    });

    expect(ws.close).toHaveBeenCalledTimes(1);
    expect(result.current.isConnected).toBe(false);
  });

  it('sends input only when the socket is open', () => {
    const { result } = renderHook(() => useInputWs('session-1'));
    const ws = MockWebSocket.instances[0];

    expect(result.current.sendInput({ type: 'key_down', key_code: 'KeyA' })).toBe(false);

    act(() => {
      ws.readyState = MockWebSocket.OPEN;
      ws.onopen?.();
    });

    expect(result.current.sendInput({ type: 'key_down', key_code: 'KeyA' })).toBe(true);
    expect(ws.send).toHaveBeenCalledWith(JSON.stringify({ type: 'key_down', key_code: 'KeyA' }));
  });

  it('reconnects after a close event', () => {
    renderHook(() => useInputWs('session-1'));
    const first = MockWebSocket.instances[0];

    act(() => {
      first.onclose?.();
    });

    expect(MockWebSocket.instances).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(2999);
    });

    expect(MockWebSocket.instances).toHaveLength(1);

    act(() => {
      vi.advanceTimersByTime(1);
    });

    expect(MockWebSocket.instances).toHaveLength(2);
    expect(MockWebSocket.instances[1].url).toContain('/api/unity/capture/sessions/session-1/input');
  });

  it('captures websocket errors', () => {
    const { result } = renderHook(() => useInputWs('session-1'));
    const ws = MockWebSocket.instances[0];

    act(() => {
      ws.onerror?.();
    });

    expect(result.current.error).toBe('WebSocket connection error');
  });
});
