import React from 'react';
import { Outlet } from 'react-router-dom';
import { Sidebar } from '../sidebar/Sidebar';
import { LuxStatusBar } from '../lux/LuxStatusBar';
import { useDashboard } from '../../hooks/useDashboard';
import type { ConnectionState } from '../../types';

interface DashboardLayoutProps {
  connectionState?: ConnectionState;
}

export const DashboardLayout: React.FC<DashboardLayoutProps> = ({ connectionState: externalState }) => {
  const {
    sidebarCollapsed,
    toggleSidebar,
    projectInfo,
    serverStatus,
  } = useDashboard();

  const effectiveState: ConnectionState = externalState
    ?? (serverStatus === 'connected' ? 'connected' : 'idle');

  return (
    <div className="lux-dashboard-container noise scanlines">
      <Sidebar 
        collapsed={sidebarCollapsed} 
        onToggleCollapse={toggleSidebar} 
      />
      
      <main className="lux-main-content" aria-label="Lux dashboard content">
        <header className="flex items-center justify-between px-6 py-4 border-b border-[var(--color-line)] bg-[var(--color-surface)]">
          <div className="flex items-center gap-3">
            {projectInfo ? (
              <>
                <h1 className="font-stencil text-[var(--text-title)] m-0">{projectInfo.name}</h1>
                <span className="sys-tag">{projectInfo.unityVersion}</span>
              </>
            ) : (
              <span className="text-[var(--color-text-muted)] font-terminal text-[var(--text-body)]">No project loaded</span>
            )}
          </div>
          
          <div className="flex items-center gap-2 font-terminal text-[var(--text-caption)]">
            <div className={`w-2 h-2 rounded-full ${effectiveState === 'connected' ? 'bg-green-500 animate-dot-pulse' : 'bg-red-500'}`} />
            <span className="uppercase tracking-widest text-[var(--color-text-muted)]">{effectiveState === 'connected' ? 'Connected' : 'Disconnected'}</span>
          </div>
        </header>
        <LuxStatusBar projectPath={projectInfo?.path ?? null} />
        
        <section className="flex-1 overflow-y-auto p-6" aria-label="Active dashboard panel">
          <Outlet />
        </section>
      </main>
    </div>
  );
};
