import React from 'react';
import type { ProjectInfo } from '../../hooks/useDashboard';

interface ProjectPanelProps {
  projectInfo: ProjectInfo | null;
  loading: boolean;
  onRefresh: () => void;
}

export const ProjectPanel: React.FC<ProjectPanelProps> = ({ projectInfo, loading, onRefresh }) => {
  return (
    <div className="flex flex-col gap-6 h-full">
      <section aria-label="Project Settings" className="lux-panel">
        <header className="lux-panel-header">
          <h2 className="font-stencil text-[var(--text-title)] m-0">Project Settings</h2>
          <button 
            type="button"
            className="px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)] transition-colors disabled:opacity-50 disabled:cursor-not-allowed" 
            onClick={onRefresh} 
            disabled={loading}
          >
            {loading ? 'Refreshing...' : 'Refresh Info'}
          </button>
        </header>
        
        <div className="lux-panel-body">
          {projectInfo ? (
            <div className="flex flex-col gap-6">
              <div className="grid grid-cols-[150px_1fr] gap-y-4 gap-x-2 items-center font-terminal text-[var(--text-body)]">
                <div className="text-[var(--color-text-muted)]">Project Name:</div>
                <div className="font-bold text-lg text-[var(--color-text)]">{projectInfo.name}</div>
                
                <div className="text-[var(--color-text-muted)]">Path:</div>
                <div className="bg-[var(--color-surface-raised)] px-3 py-1.5 rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] text-[var(--text-caption)] break-all">
                  {projectInfo.path}
                </div>
                
                <div className="text-[var(--color-text-muted)]">Unity Version:</div>
                <div className="text-[var(--color-text)]">{projectInfo.unityVersion}</div>
                
                <div className="text-[var(--color-text-muted)]">Editor Status:</div>
                <div><span className="sys-tag border-green-500/30 text-green-400 bg-green-500/10">Running</span></div>
              </div>
              
              <div className="pt-6 border-t border-[var(--color-line)]">
                <button 
                  type="button"
                  className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 transition-colors" 
                  onClick={() => console.log('Open in Explorer')}
                >
                  Open in Explorer
                </button>
              </div>
            </div>
          ) : (
            <div className="text-[var(--color-text-muted)] p-8 text-center font-terminal text-[var(--text-body)] italic">
              {loading ? 'Loading project information...' : 'No project detected. Please ensure Unity is running.'}
            </div>
          )}
        </div>
      </section>
      
      <section aria-label="Recent Projects" className="lux-panel">
        <header className="lux-panel-header">
          <h3 className="font-stencil text-[var(--text-title)] m-0">Recent Projects</h3>
        </header>
        <div className="lux-panel-body p-0">
          <ul className="list-none p-0 m-0 font-terminal text-[var(--text-body)]">
            <li className="p-4 border-b border-[var(--color-line)] flex justify-between items-center hover:bg-[var(--color-surface-raised)] transition-colors cursor-pointer">
              <span className="text-[var(--color-text)]">Neon Glitch</span>
              <span className="text-[var(--color-text-muted)] text-[var(--text-caption)]">E:\git\linalab\neon-glitch</span>
            </li>
            <li className="p-4 flex justify-between items-center hover:bg-[var(--color-surface-raised)] transition-colors cursor-pointer">
              <span className="text-[var(--color-text)]">Lux Demo</span>
              <span className="text-[var(--color-text-muted)] text-[var(--text-caption)]">E:\git\linalab\lux-demo</span>
            </li>
          </ul>
        </div>
      </section>
    </div>
  );
};
