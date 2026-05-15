import { useState } from 'react';

interface ProjectDetails {
  root: string;
  editor_version: string;
  project_name: string;
  unity_hub_path: string | null;
  unity_install_path: string | null;
  matching_editor: string | null;
}

interface ProjectSelectorProps {
  onProjectAttached?: () => void;
}

export function ProjectSelector({ onProjectAttached }: ProjectSelectorProps) {
  const [path, setPath] = useState('');
  const [projectDetails, setProjectDetails] = useState<ProjectDetails | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const handleDetect = async (projectPath: string = path) => {
    if (!projectPath) return;
    
    setLoading(true);
    setError(null);
    setProjectDetails(null);
    
    try {
      const res = await fetch('/api/project/detect', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: projectPath })
      });
      
      if (!res.ok) {
        throw new Error(`Failed to detect project: ${res.statusText}`);
      }
      
      const data = await res.json();
      if (data) {
        setProjectDetails(data);
      } else {
        setError('No Unity project detected at this path');
      }
    } catch (err) {
      console.error('Detection error:', err);
      setError(err instanceof Error ? err.message : 'Failed to detect project');
    } finally {
      setLoading(false);
    }
  };

  const handleAttach = async () => {
    if (!projectDetails) return;
    
    setLoading(true);
    setError(null);
    
    try {
      const installRes = await fetch('/api/bridge/install', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ path: projectDetails.root })
      });
      
      if (!installRes.ok) throw new Error('Bridge install failed');
      
      const compileRes = await fetch('/api/compile', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_path: projectDetails.root })
      });
      
      if (!compileRes.ok) throw new Error('Compile failed');
      
      if (onProjectAttached) {
        onProjectAttached();
      }
    } catch (err) {
      console.error('Attach error:', err);
      setError(err instanceof Error ? err.message : 'Failed to attach to project');
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="bg-[#0A0A0A] border border-gray-800 rounded-lg p-6 max-w-2xl w-full mx-auto text-white font-sans">
      <h2 className="text-xl font-semibold mb-6 flex items-center gap-2">
        <span role="img" aria-label="folder">📂</span> Select Unity Project
      </h2>
      
      <div className="flex gap-2 mb-6">
        <input
          type="text"
          value={path}
          onChange={(e) => setPath(e.target.value)}
          placeholder="/path/to/project"
          className="flex-1 bg-[#050505] border border-gray-700 rounded px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
          onKeyDown={(e) => e.key === 'Enter' && handleDetect()}
        />
        <button
          onClick={() => handleDetect()}
          disabled={!path || loading}
          className="bg-gray-800 hover:bg-gray-700 border border-gray-700 rounded px-4 py-2 text-sm transition-colors disabled:opacity-50"
        >
          Detect
        </button>
      </div>

      {loading && (
        <div className="text-gray-400 text-sm mb-4 animate-pulse">
          Processing...
        </div>
      )}

      {error && (
        <div className="bg-red-900/20 border border-red-900/50 text-red-400 p-3 rounded mb-4 text-sm">
          {error}
        </div>
      )}

      {projectDetails && (
        <div className="bg-[#050505] border border-gray-800 rounded p-4 mb-6">
          <h3 className="text-sm font-medium text-gray-400 mb-3 uppercase tracking-wider">Detected Project</h3>
          <div className="space-y-2 text-sm">
            <div className="flex">
              <span className="text-gray-500 w-24">Name:</span>
              <span className="font-medium">{projectDetails.project_name}</span>
            </div>
            <div className="flex">
              <span className="text-gray-500 w-24">Editor:</span>
              <span className="font-medium">{projectDetails.editor_version}</span>
            </div>
            <div className="flex">
              <span className="text-gray-500 w-24">Hub:</span>
              <span className="font-medium truncate" title={projectDetails.unity_hub_path || 'Not found'}>
                {projectDetails.unity_hub_path || 'Not found'}
              </span>
            </div>
            <div className="flex">
              <span className="text-gray-500 w-24">Install:</span>
              <span className="font-medium">
                {projectDetails.matching_editor ? '✅ Found' : '❌ Not found'}
              </span>
            </div>
          </div>
        </div>
      )}

      <button
        onClick={handleAttach}
        disabled={!projectDetails || loading}
        className="w-full bg-blue-600 hover:bg-blue-500 text-white font-medium py-2 px-4 rounded transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Attach Project
      </button>
    </div>
  );
}
