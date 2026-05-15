import React, { useState, useEffect } from 'react';
import { compileProject } from '../../lib/api';

export const CompilePanel: React.FC = () => {
  const [status, setStatus] = useState<'idle' | 'compiling' | 'success' | 'error'>('idle');
  const [logs, setLogs] = useState<string[]>([]);
  const [duration, setDuration] = useState(0);
  const [errorCount, setErrorCount] = useState(0);

  useEffect(() => {
    let interval: number;
    if (status === 'compiling') {
      interval = window.setInterval(() => {
        setDuration(prev => prev + 1);
      }, 1000);
    }
    return () => clearInterval(interval);
  }, [status]);

  const handleCompile = async () => {
    setStatus('compiling');
    setLogs(['Starting compilation...']);
    setDuration(0);
    setErrorCount(0);
    
    try {
      setTimeout(() => setLogs(prev => [...prev, 'Resolving dependencies...']), 1000);
      setTimeout(() => setLogs(prev => [...prev, 'Compiling scripts...']), 2000);
      
      await compileProject();
      
      setLogs(prev => [...prev, 'Compilation successful!']);
      setStatus('success');
    } catch (err) {
      setLogs(prev => [...prev, `Error: ${err instanceof Error ? err.message : 'Unknown error'}`]);
      setStatus('error');
      setErrorCount(1);
    }
  };

  return (
    <section aria-label="Compilation Panel" className="lux-panel h-full flex flex-col">
      <header className="lux-panel-header">
        <h2 className="font-stencil text-[var(--text-title)] m-0">Compilation</h2>
        <div className="flex gap-3 items-center">
          {status !== 'idle' && (
            <span className="font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">Time: {duration}s</span>
          )}
          {status === 'error' && (
            <span className="sys-tag border-red-500/30 text-red-400 bg-red-500/10">{errorCount} Errors</span>
          )}
          {status === 'success' && (
            <span className="sys-tag border-green-500/30 text-green-400 bg-green-500/10">Success</span>
          )}
          {status === 'compiling' && (
            <span className="sys-tag border-yellow-500/30 text-yellow-400 bg-yellow-500/10 animate-pulse">Compiling...</span>
          )}
          <button 
            type="button"
            className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" 
            onClick={handleCompile}
            disabled={status === 'compiling'}
          >
            {status === 'compiling' ? 'Compiling...' : 'Compile Project'}
          </button>
        </div>
      </header>
      
      <div className="lux-panel-body flex-1 bg-[#000000] font-terminal text-[var(--text-caption)] p-4 overflow-y-auto">
        {logs.length === 0 ? (
          <div className="text-[var(--color-text-muted)] italic">Ready to compile.</div>
        ) : (
          <div className="flex flex-col gap-1">
            {logs.map((log, i) => (
              <div key={i} className={`${log.includes('Error') ? 'text-red-400' : log.includes('success') ? 'text-green-400' : 'text-[var(--color-text-muted)]'}`}>
                <span className="opacity-50 mr-2">[{new Date().toLocaleTimeString().split(' ')[0]}]</span>
                {log}
              </div>
            ))}
            {status === 'compiling' && (
              <div className="animate-blink-cursor">_</div>
            )}
          </div>
        )}
      </div>
    </section>
  );
};
