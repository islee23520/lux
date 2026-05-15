export interface SkillDiscoveryEntry {
  name: string;
  version?: string;
  description?: string;
  skill_type?: string;
  scope: string;
  directory_path: string;
  manifest: Record<string, unknown>;
}

export async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const response = await fetch(`/api/${command}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(args || {}),
  });
  
  if (!response.ok) {
    throw new Error(`API error: ${response.statusText}`);
  }
  
  return response.json();
}

export const compileProject = () => invoke<void>('compile_project');
export const runTests = (mode: 'EditMode' | 'PlayMode') => invoke<void>('run_tests', { mode });
export const detectProject = async () => {
  const project = await invoke<{ root: string; project_name: string; editor_version: string } | null>('detect_project');
  return project
    ? { name: project.project_name, path: project.root, unityVersion: project.editor_version }
    : null;
};
export async function apiGet<T>(path: string): Promise<T> {
  const response = await fetch(path);

  if (!response.ok) {
    throw new Error(`API error: ${response.statusText}`);
  }

  return response.json();
}

export const listSkills = () => apiGet<SkillDiscoveryEntry[]>('/api/skills');
export const getConfig = () => invoke<unknown>('get_config');
