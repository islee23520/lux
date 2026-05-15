import React from 'react';
import { NavLink } from 'react-router-dom';

interface SidebarProps {
  collapsed: boolean;
  onToggleCollapse: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  collapsed,
  onToggleCollapse
}) => {
  const navItems = [
    { path: '/dashboard', label: 'Dashboard', icon: '📊' },
    { path: '/workbench', label: 'Workbench', icon: '🧠' },
    { path: '/kanban', label: 'Kanban', icon: '📊' },
    { path: '/progress', label: 'Progress', icon: '📈' },
    { path: '/play', label: 'Play', icon: '🎮' },
    { path: '/terminal', label: 'Terminal', icon: '💻' },
    { path: '/logs', label: 'Play Logs', icon: '📜' },
    { path: '/compile', label: 'Compile', icon: '🔨' },
    { path: '/test', label: 'Tests', icon: '🧪' },
    { path: '/log', label: 'AI Log', icon: '📝' },
    { path: '/skills', label: 'Skills', icon: '🧩' },
    { path: '/sessions', label: 'Sessions', icon: '💬' },
    { path: '/tools', label: 'Tools', icon: '🛠️' },
    { path: '/unity-run', label: 'Unity Run', icon: '🎮' },
    { path: '/loop-control', label: 'Loop Control', icon: '∞' },
  ];

  return (
    <nav 
      className={`lux-sidebar transition-all duration-300 ${collapsed ? 'w-16' : 'w-64'}`}
      aria-label="Main navigation"
    >
      <div className="h-16 flex items-center justify-center border-b border-[var(--color-line)] bg-[var(--color-surface-raised)]">
        <span className="font-stencil text-[var(--text-title)] tracking-widest">
          {collapsed ? 'L' : 'LUX OS'}
        </span>
      </div>
      
      <div id="sidebar-nav" className="flex-1 overflow-y-auto py-4 flex flex-col gap-1 px-2">
        {navItems.map(item => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) => 
              `flex items-center gap-3 px-3 py-2 rounded-md transition-colors font-terminal text-[var(--text-body)]
              ${isActive 
                ? 'bg-[var(--color-text)] text-[var(--color-bg)]' 
                : 'text-[var(--color-text-muted)] hover:bg-[var(--color-surface-raised)] hover:text-[var(--color-text)]'
              }`
            }
            title={collapsed ? item.label : undefined}
            aria-label={item.label}
          >
            <span className="text-lg flex-shrink-0 flex items-center justify-center w-6">{item.icon}</span>
            {!collapsed && <span className="truncate">{item.label}</span>}
          </NavLink>
        ))}
        <div className="mt-4 px-3 py-3 rounded-md border border-[var(--color-line)] bg-[var(--color-surface)] font-terminal text-[var(--text-micro)] text-[var(--color-text-muted)]" title="Spec → Build → Play → Feedback → Improve">
          {collapsed ? (
            <span aria-label="Lux loop">∞</span>
          ) : (
            <div className="flex flex-col gap-2">
              <span className="uppercase tracking-widest text-[var(--color-text-dim)]">Lux Loop</span>
              <span>Spec → Build → Play → Feedback → Improve</span>
            </div>
          )}
        </div>
      </div>
      
      <div className="p-4 border-t border-[var(--color-line)] flex items-center justify-between">
        <button 
          type="button"
          className="p-2 rounded-md text-[var(--color-text-muted)] hover:bg-[var(--color-surface-raised)] hover:text-[var(--color-text)] transition-colors"
          onClick={onToggleCollapse} 
          title={collapsed ? "Expand" : "Collapse"}
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          aria-expanded={!collapsed}
          aria-controls="sidebar-nav"
        >
          {collapsed ? '▶' : '◀'}
        </button>
        {!collapsed && <div className="font-terminal text-[var(--text-micro)] text-[var(--color-text-dim)]">v0.1.0</div>}
      </div>
    </nav>
  );
};
