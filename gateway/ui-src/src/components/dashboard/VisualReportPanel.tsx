import React, { useState } from 'react';

export const VisualReportPanel: React.FC = () => {
  const [status, setStatus] = useState<'idle' | 'capturing' | 'comparing' | 'done'>('idle');
  const [matchPercentage, setMatchPercentage] = useState<number | null>(null);

  const handleCapture = () => {
    setStatus('capturing');
    setTimeout(() => {
      setStatus('comparing');
      setTimeout(() => {
        setMatchPercentage(98.5);
        setStatus('done');
      }, 1500);
    }, 1000);
  };

  return (
    <section className="panel-container" aria-label="Visual regression report">
      <section className="panel-card" aria-label="Visual regression panel">
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '16px' }}>
          <h2 className="panel-title" style={{ margin: 0, border: 'none' }}>Visual Regression Report</h2>
          <button 
            className="btn btn-primary" 
            onClick={handleCapture}
            disabled={status === 'capturing' || status === 'comparing'}
          >
            {status === 'capturing' ? 'Capturing...' : status === 'comparing' ? 'Comparing...' : 'Run Visual Check'}
          </button>
        </header>
        
        {status === 'done' && matchPercentage !== null && (
          <section style={{ marginBottom: '24px', padding: '16px', backgroundColor: matchPercentage > 95 ? 'rgba(52, 211, 153, 0.1)' : 'rgba(251, 113, 133, 0.1)', border: `1px solid ${matchPercentage > 95 ? 'var(--green)' : 'var(--red)'}`, borderRadius: '6px', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }} aria-label="Visual check result">
            <div>
              <h3 style={{ margin: '0 0 8px 0', color: matchPercentage > 95 ? 'var(--green)' : 'var(--red)' }}>
                {matchPercentage > 95 ? 'Visual Check Passed' : 'Visual Check Failed'}
              </h3>
              <div style={{ color: 'var(--muted)' }}>Baseline match: {matchPercentage}%</div>
            </div>
            <div style={{ fontSize: '2em', fontWeight: 'bold', color: matchPercentage > 95 ? 'var(--green)' : 'var(--red)' }}>
              {matchPercentage}%
            </div>
          </section>
        )}
        
        <div className="grid-2">
          <div style={{ border: '1px solid var(--line)', borderRadius: '4px', overflow: 'hidden' }}>
            <div style={{ padding: '8px', backgroundColor: 'rgba(15, 23, 42, 0.8)', borderBottom: '1px solid var(--line)', textAlign: 'center', fontWeight: 'bold' }}>
              Baseline
            </div>
            <div style={{ height: '250px', backgroundColor: 'rgba(15, 23, 42, 0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--muted)' }}>
              [Baseline Image Placeholder]
            </div>
          </div>
          
          <div style={{ border: '1px solid var(--line)', borderRadius: '4px', overflow: 'hidden' }}>
            <div style={{ padding: '8px', backgroundColor: 'rgba(15, 23, 42, 0.8)', borderBottom: '1px solid var(--line)', textAlign: 'center', fontWeight: 'bold' }}>
              Current
            </div>
            <div style={{ height: '250px', backgroundColor: 'rgba(15, 23, 42, 0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--muted)' }}>
              {status === 'idle' ? 'Run check to capture current state' : '[Current Image Placeholder]'}
            </div>
          </div>
        </div>
        
        {status === 'done' && (
          <div style={{ marginTop: '16px', border: '1px solid var(--line)', borderRadius: '4px', overflow: 'hidden' }}>
            <div style={{ padding: '8px', backgroundColor: 'rgba(15, 23, 42, 0.8)', borderBottom: '1px solid var(--line)', textAlign: 'center', fontWeight: 'bold' }}>
              Diff Overlay
            </div>
            <div style={{ height: '300px', backgroundColor: 'rgba(15, 23, 42, 0.4)', display: 'flex', alignItems: 'center', justifyContent: 'center', color: 'var(--muted)' }}>
              [Diff Image Placeholder]
            </div>
          </div>
        )}
      </section>
    </section>
  );
};
