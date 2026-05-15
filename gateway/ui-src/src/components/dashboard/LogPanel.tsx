import React, { useState, useEffect, useRef } from 'react';

interface LogEntry {
  id: number;
  timestamp: Date;
  level: 'info' | 'warning' | 'error';
  message: string;
}

export const LogPanel: React.FC = () => {
  const [logs, setLogs] = useState<LogEntry[]>([]);
  const [levelFilter, setLevelFilter] = useState<'all' | 'info' | 'warning' | 'error'>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [autoScroll, setAutoScroll] = useState(true);
  const logEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const interval = setInterval(() => {
      const levels: ('info' | 'warning' | 'error')[] = ['info', 'info', 'info', 'warning', 'error'];
      const randomLevel = levels[Math.floor(Math.random() * levels.length)];
      
      const newLog: LogEntry = {
        id: Date.now(),
        timestamp: new Date(),
        level: randomLevel,
        message: `System log message generated at ${new Date().toISOString()}`
      };
      
      setLogs(prev => [...prev.slice(-99), newLog]);
    }, 5000);
    
    return () => clearInterval(interval);
  }, []);

  useEffect(() => {
    if (autoScroll && logEndRef.current) {
      logEndRef.current.scrollIntoView({ behavior: 'smooth' });
    }
  }, [logs, autoScroll]);

  const filteredLogs = logs.filter(log => {
    if (levelFilter !== 'all' && log.level !== levelFilter) return false;
    if (searchQuery && !log.message.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    return true;
  });

  return (
    <section aria-label="System Logs Panel" className="lux-panel h-full flex flex-col">
      <header className="lux-panel-header">
        <h2 className="font-stencil text-[var(--text-title)] m-0">System Logs</h2>
        <div className="flex gap-3 items-center">
          <input 
            type="text" 
            placeholder="Search logs..." 
            className="bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] px-3 py-2 rounded-sm font-terminal text-[var(--text-caption)] focus:outline-none focus:border-[var(--color-line-strong)] w-48"
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
          />
          <select 
            className="bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] px-3 py-2 rounded-sm font-terminal text-[var(--text-caption)] focus:outline-none focus:border-[var(--color-line-strong)]" 
            value={levelFilter} 
            onChange={(e) => setLevelFilter(e.target.value as any)}
          >
            <option value="all">All Levels</option>
            <option value="info">Info</option>
            <option value="warning">Warning</option>
            <option value="error">Error</option>
          </select>
          <label className="flex items-center gap-2 font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)] cursor-pointer hover:text-[var(--color-text)] transition-colors">
            <input 
              type="checkbox" 
              className="accent-[var(--color-text)]"
              checked={autoScroll} 
              onChange={(e) => setAutoScroll(e.target.checked)} 
            />
            Auto-scroll
          </label>
          <button 
            type="button"
            className="px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border border-[var(--color-line)] text-[var(--color-text-muted)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)] transition-colors" 
            onClick={() => setLogs([])}
          >
            Clear
          </button>
        </div>
      </header>
      
      <div className="lux-panel-body flex-1 bg-[#000000] font-terminal text-[var(--text-caption)] p-4 overflow-y-auto">
        {filteredLogs.length === 0 ? (
          <div className="text-[var(--color-text-muted)] italic text-center mt-24">
            No logs match the current filters.
          </div>
        ) : (
          <div className="flex flex-col gap-1">
            {filteredLogs.map(log => (
              <div key={log.id} className="flex gap-3 items-start hover:bg-white/5 p-1 rounded-sm transition-colors">
                <span className="opacity-50 flex-shrink-0">[{log.timestamp.toLocaleTimeString()}]</span>
                <span className={`sys-tag flex-shrink-0 w-16 justify-center ${
                  log.level === 'error' ? 'border-red-500/30 text-red-400 bg-red-500/10' : 
                  log.level === 'warning' ? 'border-yellow-500/30 text-yellow-400 bg-yellow-500/10' : 
                  'border-blue-500/30 text-blue-400 bg-blue-500/10'
                }`}>
                  {log.level.toUpperCase()}
                </span>
                <span className={`break-all ${
                  log.level === 'error' ? 'text-red-400' : 
                  log.level === 'warning' ? 'text-yellow-400' : 
                  'text-[var(--color-text)]'
                }`}>
                  {log.message}
                </span>
              </div>
            ))}
            <div ref={logEndRef} />
          </div>
        )}
      </div>
    </section>
  );
};
