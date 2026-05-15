import { useCallback, useEffect, useRef, useState } from 'react';

export type SpecDomainKey = 'design' | 'architecture' | 'art-style' | 'audio' | 'narrative' | 'levels' | 'ui-ux';

export interface DomainSpec {
  name: string;
  content_path: string;
  fields: Record<string, unknown>;
  ambiguity_score: number;
  last_evaluated: string | null;
  defined: boolean;
}

export interface SpecDomains {
  design: DomainSpec | null;
  architecture: DomainSpec | null;
  art_style: DomainSpec | null;
  audio: DomainSpec | null;
  narrative: DomainSpec | null;
  levels: DomainSpec | null;
  ui_ux: DomainSpec | null;
  custom: Record<string, DomainSpec>;
}

export type PillarStatus = 'Strong' | 'NeedsWork' | 'Missing';

export interface PhaseResult {
  name: string;
  status: PillarStatus;
  summary: string | null;
  score: number;
  questions: string[];
}

export interface PillarRating {
  status: PillarStatus;
  description: string | null;
  score: number;
}

export interface TetradResult {
  mechanics: PillarRating;
  story: PillarRating;
  aesthetics: PillarRating;
  technology: PillarRating;
  harmony_score: number;
}

export interface AssessmentResult {
  status: PillarStatus;
  viability_score: number;
  strengths: string[];
  risks: string[];
  recommendations: string[];
  summary: string | null;
}

export interface SchellEvaluation {
  phase1_experience: PhaseResult;
  phase2_tetrad: TetradResult;
  phase3_core_loop: PhaseResult;
  phase4_motivation: PhaseResult;
  phase5_assessment: AssessmentResult;
}

export interface SpecProject {
  version: string;
  project_id: string;
  project_name: string;
  created_at: string;
  updated_at: string;
  source: string;
  status: 'Draft' | 'Active' | 'Deprecated';
  domains: SpecDomains;
  schell_evaluation: SchellEvaluation;
  overall_ambiguity: number;
}

export interface DomainContentResponse {
  domain: string;
  content: string;
}

export interface AmbiguityReport {
  overall: number;
  domains: Record<string, number>;
}

interface UseSpecState {
  spec: SpecProject | null;
  ambiguity: AmbiguityReport | null;
  loading: boolean;
  saving: boolean;
  wsConnected: boolean;
  error: string | null;
}

const buildQuery = (projectPath: string) => `project_path=${encodeURIComponent(projectPath)}`;

async function readJson<T>(response: Response): Promise<T> {
  if (!response.ok) {
    const message = await response.text();
    throw new Error(message || `API error: ${response.status} ${response.statusText}`);
  }
  return response.json() as Promise<T>;
}

export async function fetchSpec(projectPath: string): Promise<SpecProject> {
  const response = await fetch(`/api/lux/spec?${buildQuery(projectPath)}`);
  return readJson<SpecProject>(response);
}

export async function fetchSpecDomain(projectPath: string, domain: SpecDomainKey): Promise<DomainContentResponse> {
  const response = await fetch(`/api/lux/spec/${domain}?${buildQuery(projectPath)}`);
  return readJson<DomainContentResponse>(response);
}

export async function updateSpecDomain(
  projectPath: string,
  domain: SpecDomainKey,
  content: string,
): Promise<void> {
  const response = await fetch(`/api/lux/spec/${domain}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ project_path: projectPath, content }),
  });
  await readJson<{ ok: boolean }>(response);
}

export async function fetchSpecAmbiguity(projectPath: string): Promise<AmbiguityReport> {
  const response = await fetch(`/api/lux/spec/ambiguity?${buildQuery(projectPath)}`);
  return readJson<AmbiguityReport>(response);
}

export function useSpec(projectPath: string | null) {
  const [state, setState] = useState<UseSpecState>({
    spec: null,
    ambiguity: null,
    loading: false,
    saving: false,
    wsConnected: false,
    error: null,
  });
  const refreshRef = useRef<() => Promise<void>>(async () => undefined);

  const refresh = useCallback(async () => {
    if (!projectPath) {
      setState(prev => ({ ...prev, spec: null, ambiguity: null, loading: false }));
      return;
    }

    setState(prev => ({ ...prev, loading: true, error: null }));
    try {
      const [nextSpec, nextAmbiguity] = await Promise.all([
        fetchSpec(projectPath),
        fetchSpecAmbiguity(projectPath),
      ]);
      setState(prev => ({ ...prev, spec: nextSpec, ambiguity: nextAmbiguity, loading: false, error: null }));
    } catch (err) {
      setState(prev => ({
        ...prev,
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      }));
    }
  }, [projectPath]);

  useEffect(() => {
    refreshRef.current = refresh;
  }, [refresh]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const loadDomain = useCallback(async (domain: SpecDomainKey) => {
    if (!projectPath) {
      throw new Error('Project path is required');
    }
    return fetchSpecDomain(projectPath, domain);
  }, [projectPath]);

  const saveDomain = useCallback(async (domain: SpecDomainKey, content: string) => {
    if (!projectPath) {
      throw new Error('Project path is required');
    }

    setState(prev => ({ ...prev, saving: true, error: null }));
    try {
      await updateSpecDomain(projectPath, domain, content);
      await refreshRef.current();
      setState(prev => ({ ...prev, saving: false, error: null }));
    } catch (err) {
      setState(prev => ({
        ...prev,
        saving: false,
        error: err instanceof Error ? err.message : String(err),
      }));
      throw err;
    }
  }, [projectPath]);

  return {
    ...state,
    refresh,
    loadDomain,
    saveDomain,
  };
}
