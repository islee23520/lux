import { useState, useEffect, useCallback } from 'react';
import { detectProject } from '../lib/api';

export type PanelType = 'overview' | 'compile' | 'tests' | 'logs' | 'project' | 'skills' | 'visual-report';

export interface ProjectInfo {
  name: string;
  path: string;
  unityVersion: string;
}

export function useDashboard() {
  const [activePanel, setActivePanel] = useState<PanelType>('overview');
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [projectInfo, setProjectInfo] = useState<ProjectInfo | null>(null);
  const [serverStatus, setServerStatus] = useState<'connected' | 'disconnected'>('disconnected');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const toggleSidebar = useCallback(() => {
    setSidebarCollapsed(prev => !prev);
  }, []);

  const fetchProjectInfo = useCallback(async () => {
    try {
      setLoading(true);
      const info = await detectProject();
      setProjectInfo(info);
      setServerStatus('connected');
      setError(null);
    } catch (err) {
      console.error('Failed to fetch project info:', err);
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    fetchProjectInfo();

    const interval = setInterval(() => {
      fetchProjectInfo();
    }, 30000);

    return () => clearInterval(interval);
  }, [fetchProjectInfo]);

  useEffect(() => {
    const checkHealth = async () => {
      try {
        const res = await fetch('/api/health');
        if (!res.ok) {
          setServerStatus('disconnected');
          return;
        }
        setServerStatus('connected');
      } catch {
        setServerStatus('disconnected');
      }
    };
    checkHealth();
    const interval = setInterval(checkHealth, 10000);
    return () => clearInterval(interval);
  }, []);

  return {
    activePanel,
    setActivePanel,
    sidebarCollapsed,
    toggleSidebar,
    projectInfo,
    serverStatus,
    loading,
    error,
    refreshProjectInfo: fetchProjectInfo
  };
}
