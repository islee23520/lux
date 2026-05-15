import { describe, it, expect } from 'vitest'
import type {
  UnitySpec,
  TargetsSpec,
  PackageEntry,
  PackagesSpec,
  TestingSpec,
  GlossarySpec,
  DomainSpec,
  PillarStatus,
  PillarRating,
  PhaseResult,
  TetradResult,
  AssessmentResult,
  SchellEvaluation,
  LuxSpecProject,
  LuxEvalResult,
  LuxPluginConfig,
  LuxGlossaryEntry
} from '../types'
import type {
  OrchestratorConfig,
  OrchestratorDeps,
  StopReason,
  CycleResult
} from '../index'

describe('Lux Plugin Types', () => {
  describe('OrchestratorConfig', () => {
    it('should allow valid OrchestratorConfig', () => {
      const config: OrchestratorConfig = {
        projectPath: '/path/to/project',
        gatewayUrl: 'http://localhost:18766',
        maxContinuations: 10,
        minContinuationIntervalMs: 3000,
        healthThreshold: 20,
        maxStagnation: 3
      }
      expect(config.projectPath).toBe('/path/to/project')
    })

    it('should allow minimal OrchestratorConfig', () => {
      const config: OrchestratorConfig = {
        projectPath: '/path/to/project',
        gatewayUrl: 'http://localhost:18766'
      }
      expect(config.projectPath).toBe('/path/to/project')
    })
  })

  describe('OrchestratorDeps', () => {
    it('should allow valid OrchestratorDeps', () => {
      const deps: OrchestratorDeps = {
        stateClient: {
          readContinuationState: async () => ({ status: 'Active' } as any),
          writeContinuationState: async () => ({ seq: 1 })
        },
        ticketLoader: {
          loadTickets: () => ({ tickets: [] } as any),
          invalidateCache: () => {}
        },
        signalIntegrator: () => undefined
      }
      expect(deps.signalIntegrator()).toBeUndefined()
    })
  })

  describe('StopReason', () => {
    it('should allow all valid stop reasons', () => {
      const reasons: StopReason[] = [
        'max_continations',
        'user_abort',
        'stagnation',
        'health_critical',
        'all_complete',
        'ambiguity_too_high',
        'consecutive_state_error',
        null
      ]
      expect(reasons).toHaveLength(8)
    })
  })

  describe('CycleResult', () => {
    it('should allow valid CycleResult', () => {
      const result: CycleResult = {
        dispatched: true,
        stopReason: 'max_continations',
        selectedTicketId: 'T-123',
        message: 'Continuing with T-123'
      }
      expect(result.dispatched).toBe(true)
    })
  })

  describe('UnitySpec', () => {
    it('should allow valid UnitySpec object', () => {
      const spec: UnitySpec = {
        required_version: '2022.3.0f1',
        detected_version: '2022.3.0f1',
        render_pipeline: 'urp',
        scripting_backend: 'mono'
      }
      expect(spec.render_pipeline).toBe('urp')
    })

    it('should allow null values for optional-like fields', () => {
      const spec: UnitySpec = {
        required_version: null,
        detected_version: null,
        render_pipeline: null,
        scripting_backend: null
      }
      expect(spec.required_version).toBeNull()
    })
  })

  describe('TargetsSpec', () => {
    it('should allow valid TargetsSpec object', () => {
      const spec: TargetsSpec = {
        platforms: ['iOS', 'Android'],
        min_sdk: { iOS: '15.0', Android: '31' },
        test_platform: 'iOS'
      }
      expect(spec.platforms).toContain('iOS')
    })
  })

  describe('PackagesSpec', () => {
    it('should allow valid PackagesSpec with entries', () => {
      const entry: PackageEntry = {
        name: 'com.unity.render-pipelines.universal',
        reason: 'Required for URP',
        version: '14.0.0'
      }
      const spec: PackagesSpec = {
        required: [entry],
        forbidden: [],
        detected: [entry]
      }
      expect(spec.required[0].name).toBe('com.unity.render-pipelines.universal')
    })
  })

  describe('TestingSpec', () => {
    it('should allow valid TestingSpec', () => {
      const spec: TestingSpec = {
        framework: 'Unity Test Framework',
        strategy: 'Unit + Integration',
        coverage: true
      }
      expect(spec.coverage).toBe(true)
    })
  })

  describe('DomainSpec', () => {
    it('should allow valid DomainSpec', () => {
      const spec: DomainSpec = {
        name: 'Architecture',
        content_path: 'docs/arch/',
        fields: { pattern: 'MVC' },
        ambiguity_score: 0.5,
        last_evaluated: '2026-05-13T00:00:00Z',
        defined: true
      }
      expect(spec.defined).toBe(true)
    })
  })

  describe('SchellEvaluation and supporting types', () => {
    it('should allow valid SchellEvaluation structure', () => {
      const pillar: PillarRating = {
        status: 'Strong',
        description: 'Good',
        score: 0.9
      }
      const phase: PhaseResult = {
        name: 'Experience',
        status: 'Strong',
        summary: 'Excellent',
        score: 0.9,
        questions: ['Is it fun?']
      }
      const tetrad: TetradResult = {
        mechanics: pillar,
        story: pillar,
        aesthetics: pillar,
        technology: pillar,
        harmony_score: 0.9
      }
      const assessment: AssessmentResult = {
        status: 'Strong',
        viability_score: 0.9,
        strengths: ['Innovation'],
        risks: [],
        recommendations: [],
        summary: 'Proceed'
      }

      const evalResult: SchellEvaluation = {
        phase1_experience: phase,
        phase2_tetrad: tetrad,
        phase3_core_loop: phase,
        phase4_motivation: phase,
        phase5_assessment: assessment
      }
      expect(evalResult.phase2_tetrad.mechanics.status).toBe('Strong')
    })
  })

  describe('LuxSpecProject', () => {
    it('should allow a full project specification', () => {
      const project: LuxSpecProject = {
        version: '1.0.0',
        project_id: 'test-id',
        project_name: 'Test Project',
        created_at: '2026-05-13T00:00:00Z',
        updated_at: '2026-05-13T00:00:00Z',
        source: 'manual',
        status: 'Active',
        domains: {
          design: null,
          architecture: null,
          art_style: null,
          audio: null,
          narrative: null,
          levels: null,
          ui_ux: null,
          custom: {}
        },
        schell_evaluation: {} as SchellEvaluation,
        overall_ambiguity: 0.1,
        unity: null,
        targets: null,
        packages: null,
        testing: null,
        glossary: null
      }
      expect(project.status).toBe('Active')
    })
  })

  describe('LuxPluginConfig', () => {
    it('should allow valid plugin configuration', () => {
      const config: LuxPluginConfig = {
        maxContinuations: 10,
        specPath: '.lux/spec.json',
        glossaryPath: '.lux/glossary.md',
        targetAmbiguity: 0.05
      }
      expect(config.maxContinuations).toBe(10)
    })
  })

  describe('LuxGlossaryEntry', () => {
    it('should allow valid glossary entry', () => {
      const entry: LuxGlossaryEntry = {
        term: 'SSoT',
        definition: 'Single Source of Truth',
        context: 'Project architecture',
        first_seen: '2026-05-13T00:00:00Z'
      }
      expect(entry.term).toBe('SSoT')
    })
  })
})
