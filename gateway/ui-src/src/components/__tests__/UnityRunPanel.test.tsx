import { fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { UnityRunPanel } from '../UnityRunPanel';
import { useCapture } from '../../hooks/useCapture';
import { useDashboard } from '../../hooks/useDashboard';
import { useInputWs } from '../../hooks/useInputWs';

vi.mock('../../hooks/useCapture');
vi.mock('../../hooks/useDashboard');
vi.mock('../../hooks/useInputWs');

const mockedUseCapture = vi.mocked(useCapture);
const mockedUseDashboard = vi.mocked(useDashboard);
const mockedUseInputWs = vi.mocked(useInputWs);

describe('UnityRunPanel', () => {
  const startCapture = vi.fn();
  const stopCapture = vi.fn();
  const sendInput = vi.fn();

  beforeEach(() => {
    mockedUseDashboard.mockReturnValue({
      activePanel: 'overview',
      setActivePanel: vi.fn(),
      sidebarCollapsed: false,
      toggleSidebar: vi.fn(),
      projectInfo: { name: 'Neon Glitch', path: '/unity/project', unityVersion: '6000.0' },
      serverStatus: 'connected',
      loading: false,
      error: null,
      refreshProjectInfo: vi.fn(),
    });

    mockedUseCapture.mockReturnValue({
      sessionId: null,
      status: 'idle',
      error: null,
      sessionInfo: null,
      startCapture,
      stopCapture,
      getStatus: vi.fn(),
    });

    mockedUseInputWs.mockReturnValue({
      isConnected: false,
      error: null,
      sendInput,
    });
  });

  it('renders the idle placeholder', () => {
    render(<UnityRunPanel />);

    expect(screen.getByText('NO ACTIVE CAPTURE SESSION')).toBeInTheDocument();
    expect(screen.getByText('IDLE')).toBeInTheDocument();
  });

  it('renders the stream view while streaming', () => {
    mockedUseCapture.mockReturnValue({
      sessionId: 'session-1',
      status: 'streaming',
      error: null,
      sessionInfo: { session_id: 'session-1', status: 'streaming' },
      startCapture,
      stopCapture,
      getStatus: vi.fn(),
    });
    mockedUseInputWs.mockReturnValue({ isConnected: true, error: null, sendInput });

    render(<UnityRunPanel />);

    const stream = screen.getByAltText('Unity Live Stream');
    expect(stream).toHaveAttribute('src', '/api/unity/capture/sessions/session-1/stream');
    expect(screen.getByText('WS CONNECTED')).toBeInTheDocument();
  });

  it('starts capture with the selected project and default settings', () => {
    render(<UnityRunPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'START' }));

    expect(startCapture).toHaveBeenCalledWith('/unity/project', 1280, 720, 30);
  });

  it('stops the active capture session', () => {
    mockedUseCapture.mockReturnValue({
      sessionId: 'session-1',
      status: 'streaming',
      error: null,
      sessionInfo: { session_id: 'session-1', status: 'streaming' },
      startCapture,
      stopCapture,
      getStatus: vi.fn(),
    });

    render(<UnityRunPanel />);

    fireEvent.click(screen.getByRole('button', { name: 'STOP' }));

    expect(stopCapture).toHaveBeenCalledTimes(1);
  });

  it('allows FPS and resolution to be configured before starting', () => {
    render(<UnityRunPanel />);
    const [fpsSelect, resolutionSelect] = screen.getAllByRole('combobox');

    fireEvent.change(fpsSelect, { target: { value: '60' } });
    fireEvent.change(resolutionSelect, { target: { value: '1920x1080' } });
    fireEvent.click(screen.getByRole('button', { name: 'START' }));

    expect(startCapture).toHaveBeenCalledWith('/unity/project', 1920, 1080, 60);
  });
});
