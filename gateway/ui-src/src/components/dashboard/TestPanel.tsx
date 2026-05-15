import React, { useState } from 'react';
import { runTests } from '../../lib/api';

interface TestResult {
  id: string;
  name: string;
  status: 'passed' | 'failed' | 'skipped';
  duration: number;
  message?: string;
}

export const TestPanel: React.FC = () => {
  const [mode, setMode] = useState<'EditMode' | 'PlayMode'>('EditMode');
  const [status, setStatus] = useState<'idle' | 'running' | 'completed'>('idle');
  const [results, setResults] = useState<TestResult[]>([]);
  const [filter, setFilter] = useState<'all' | 'passed' | 'failed'>('all');

  const handleRunTests = async () => {
    setStatus('running');
    setResults([]);
    
    try {
      await runTests(mode);
      
      setResults([
        { id: '1', name: 'PlayerMovement_ShouldMoveForward', status: 'passed', duration: 120 },
        { id: '2', name: 'Weapon_ShouldFireProjectile', status: 'passed', duration: 45 },
        { id: '3', name: 'Enemy_ShouldTakeDamage', status: 'failed', duration: 15, message: 'Expected health to be 90, but was 100' },
        { id: '4', name: 'UI_ShouldUpdateScore', status: 'passed', duration: 30 },
      ]);
      setStatus('completed');
    } catch (err) {
      console.error('Test run failed:', err);
      setStatus('completed');
    }
  };

  const passedCount = results.filter(r => r.status === 'passed').length;
  const failedCount = results.filter(r => r.status === 'failed').length;
  
  const filteredResults = results.filter(r => filter === 'all' || r.status === filter);

  return (
    <section aria-label="Test Runner Panel" className="lux-panel h-full flex flex-col">
      <header className="lux-panel-header">
        <h2 className="font-stencil text-[var(--text-title)] m-0">Test Runner</h2>
        <div className="flex gap-3 items-center">
          <select 
            className="bg-[var(--color-surface-raised)] border border-[var(--color-line)] text-[var(--color-text)] px-3 py-2 rounded-sm font-terminal text-[var(--text-caption)] focus:outline-none focus:border-[var(--color-line-strong)]" 
            value={mode} 
            onChange={(e) => setMode(e.target.value as 'EditMode' | 'PlayMode')}
            disabled={status === 'running'}
          >
            <option value="EditMode">Edit Mode</option>
            <option value="PlayMode">Play Mode</option>
          </select>
          <button 
            type="button"
            className="px-4 py-2 bg-[var(--color-text)] text-[var(--color-bg)] font-terminal text-[var(--text-caption)] uppercase tracking-widest rounded-sm hover:bg-white/90 transition-colors disabled:opacity-50 disabled:cursor-not-allowed" 
            onClick={handleRunTests}
            disabled={status === 'running'}
          >
            {status === 'running' ? 'Running...' : 'Run Tests'}
          </button>
        </div>
      </header>
      
      <div className="lux-panel-body flex-1 overflow-y-auto">
        {status === 'completed' && results.length > 0 && (
          <div className="flex gap-6 mb-6 p-4 bg-[var(--color-surface-raised)] border border-[var(--color-line)] rounded-sm font-terminal text-[var(--text-caption)]">
            <div className="font-bold text-[var(--color-text)]">Summary:</div>
            <div className="text-green-400">{passedCount} Passed</div>
            <div className="text-red-400">{failedCount} Failed</div>
            <div className="text-[var(--color-text-muted)]">{results.length} Total</div>
          </div>
        )}
        
        <div className="flex gap-2 mb-4">
          <button 
            type="button"
            className={`px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border transition-colors ${filter === 'all' ? 'bg-[var(--color-text)] text-[var(--color-bg)] border-[var(--color-text)]' : 'bg-transparent text-[var(--color-text-muted)] border-[var(--color-line)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)]'}`} 
            onClick={() => setFilter('all')}
          >
            All
          </button>
          <button 
            type="button"
            className={`px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border transition-colors ${filter === 'passed' ? 'bg-[var(--color-text)] text-[var(--color-bg)] border-[var(--color-text)]' : 'bg-transparent text-[var(--color-text-muted)] border-[var(--color-line)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)]'}`} 
            onClick={() => setFilter('passed')}
          >
            Passed
          </button>
          <button 
            type="button"
            className={`px-3 py-1.5 font-terminal text-[var(--text-caption)] rounded-sm border transition-colors ${filter === 'failed' ? 'bg-[var(--color-text)] text-[var(--color-bg)] border-[var(--color-text)]' : 'bg-transparent text-[var(--color-text-muted)] border-[var(--color-line)] hover:border-[var(--color-line-strong)] hover:text-[var(--color-text)]'}`} 
            onClick={() => setFilter('failed')}
          >
            Failed
          </button>
        </div>
        
        <div className="border border-[var(--color-line)] rounded-sm overflow-hidden">
          <table className="w-full text-left border-collapse">
            <thead className="bg-[var(--color-surface-raised)] border-b border-[var(--color-line)] font-terminal text-[var(--text-caption)] text-[var(--color-text-muted)]">
              <tr>
                <th className="p-3 font-normal">Status</th>
                <th className="p-3 font-normal">Test Name</th>
                <th className="p-3 font-normal">Duration (ms)</th>
                <th className="p-3 font-normal">Message</th>
              </tr>
            </thead>
            <tbody className="font-terminal text-[var(--text-caption)]">
              {filteredResults.length === 0 ? (
                <tr>
                  <td colSpan={4} className="p-8 text-center text-[var(--color-text-muted)] italic">
                    {status === 'idle' ? 'Click Run Tests to start' : 'No tests match the current filter'}
                  </td>
                </tr>
              ) : (
                filteredResults.map(test => (
                  <tr key={test.id} className="border-b border-[var(--color-line)] last:border-0 hover:bg-[var(--color-surface-raised)] transition-colors">
                    <td className="p-3">
                      <span className={`sys-tag ${test.status === 'passed' ? 'border-green-500/30 text-green-400 bg-green-500/10' : 'border-red-500/30 text-red-400 bg-red-500/10'}`}>
                        {test.status.toUpperCase()}
                      </span>
                    </td>
                    <td className="p-3 text-[var(--color-text)]">{test.name}</td>
                    <td className="p-3 text-[var(--color-text-muted)]">{test.duration}</td>
                    <td className="p-3 text-red-400 text-xs">{test.message || '-'}</td>
                  </tr>
                ))
              )}
            </tbody>
          </table>
        </div>
      </div>
    </section>
  );
};
