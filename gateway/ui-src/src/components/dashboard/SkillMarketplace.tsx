import React, { useCallback, useMemo, useState } from 'react';
import { listSkills, type SkillDiscoveryEntry } from '../../lib/api';

interface Skill {
  id: string;
  name: string;
  version: string;
  description: string;
  installed: boolean;
  scope: string;
}

const toSkill = (entry: SkillDiscoveryEntry): Skill => ({
  id: `${entry.scope}:${entry.name}`,
  name: entry.name,
  version: entry.version ?? 'unknown',
  description: entry.description ?? '',
  installed: true,
  scope: entry.scope,
});

export const SkillMarketplace: React.FC = () => {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [tab, setTab] = useState<'installed' | 'available'>('installed');
  const [searchQuery, setSearchQuery] = useState('');

  const fetchSkills = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const entries = await listSkills();
      setSkills(entries.map(toSkill));
      setLoaded(true);
    } catch (e) {
      setError(e instanceof Error ? e.message : 'Failed to load skills');
    } finally {
      setLoading(false);
    }
  }, []);

  const filteredSkills = useMemo(() => skills.filter(skill => {
    if (tab === 'installed' && !skill.installed) return false;
    if (tab === 'available' && skill.installed) return false;
    if (searchQuery && !skill.name.toLowerCase().includes(searchQuery.toLowerCase()) && !skill.description.toLowerCase().includes(searchQuery.toLowerCase())) return false;
    return true;
  }), [searchQuery, skills, tab]);

  return (
    <section className="panel-container" aria-label="Skill marketplace">
      <section className="panel-card" aria-label="Skill marketplace browser">
        <header style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: '12px', marginBottom: '16px' }}>
          <div>
            <h2 className="panel-title" style={{ margin: 0, border: 'none' }}>Skill Marketplace</h2>
            <p style={{ color: 'var(--muted)', margin: '4px 0 0 0', fontSize: '0.9em' }}>
              Skills are read passively only when you load or refresh this view.
            </p>
          </div>
          <div style={{ display: 'flex', gap: '8px', alignItems: 'center' }}>
            <input
              type="text"
              placeholder="Search skills..."
              className="btn"
              style={{ backgroundColor: 'rgba(15, 23, 42, 0.58)', cursor: 'text', width: '250px' }}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              disabled={!loaded}
            />
            <button type="button" className="btn btn-primary" onClick={() => void fetchSkills()} disabled={loading}>
              {loaded ? 'Refresh' : 'Load skills'}
            </button>
          </div>
        </header>

        <div style={{ display: 'flex', gap: '16px', borderBottom: '1px solid var(--line)', marginBottom: '16px' }} role="tablist" aria-label="Skill filters">
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'installed'}
            style={{ padding: '8px 16px', cursor: 'pointer', borderBottom: tab === 'installed' ? '2px solid var(--blue)' : '2px solid transparent', color: tab === 'installed' ? 'var(--blue)' : 'var(--muted)' }}
            onClick={() => setTab('installed')}
          >
            Installed ({skills.filter(s => s.installed).length})
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={tab === 'available'}
            style={{ padding: '8px 16px', cursor: 'pointer', borderBottom: tab === 'available' ? '2px solid var(--blue)' : '2px solid transparent', color: tab === 'available' ? 'var(--blue)' : 'var(--muted)' }}
            onClick={() => setTab('available')}
          >
            Available ({skills.filter(s => !s.installed).length})
          </button>
        </div>

        {error ? (
          <div role="alert" style={{ textAlign: 'center', padding: '16px', color: 'var(--red)' }}>{error}</div>
        ) : null}

        {loading ? (
          <div style={{ textAlign: 'center', padding: '32px', color: 'var(--muted)' }}>Loading skills...</div>
        ) : !loaded ? (
          <div style={{ textAlign: 'center', padding: '32px', color: 'var(--muted)' }}>Skills have not been loaded yet.</div>
        ) : filteredSkills.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '32px', color: 'var(--muted)' }}>No skills found.</div>
        ) : (
          <div className="grid-2">
            {filteredSkills.map(skill => (
              <article key={skill.id} style={{ border: '1px solid var(--line)', borderRadius: '6px', padding: '16px', backgroundColor: 'rgba(15, 23, 42, 0.58)', display: 'flex', flexDirection: 'column' }}>
                <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', marginBottom: '8px' }}>
                  <h3 style={{ margin: 0, color: 'var(--text)' }}>{skill.name}</h3>
                  <span className="badge badge-info">{skill.scope} · v{skill.version}</span>
                </div>
                <p style={{ color: 'var(--muted)', fontSize: '0.9em', flex: 1, margin: '8px 0 16px 0' }}>{skill.description || 'No description provided.'}</p>
                <div style={{ display: 'flex', justifyContent: 'flex-end' }}>
                  <span className="badge badge-success">Installed</span>
                </div>
              </article>
            ))}
          </div>
        )}
      </section>
    </section>
  );
};
