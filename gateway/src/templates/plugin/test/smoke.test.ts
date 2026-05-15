import { describe, expect, it } from 'vitest'

import * as pluginIndex from '../index'
import * as compileGuard from '../compile-guard'
import * as compactionGuard from '../compaction-guard'
import * as continuationInjector from '../continuation-injector'
import * as continuationStateClient from '../continuation-state-client'
import * as errorRecovery from '../error-recovery'
import * as externalSignalIntegrator from '../external-signal-integrator'
import * as gatewaySpawn from '../gateway-spawn'
import * as glossaryManager from '../glossary-manager'
import * as luxOverlay from '../lux-overlay'
import * as nextActionGenerator from '../next-action-generator'
import * as progressPoller from '../progress-poller'
import * as promptBuilder from '../prompt-builder'
import * as sessionEndDetector from '../session-end-detector'
import * as sessionState from '../session-state'
import * as specEvaluator from '../spec-evaluator'
import * as stagnationDetection from '../stagnation-detection'
import * as stopEvaluator from '../stop-evaluator'
import * as ticketLoader from '../ticket-loader'
import * as types from '../types'

describe('plugin template smoke imports', () => {
  it('loads the major modules without import errors', () => {
    const modules = [
      pluginIndex,
      compileGuard,
      compactionGuard,
      continuationInjector,
      continuationStateClient,
      errorRecovery,
      externalSignalIntegrator,
      gatewaySpawn,
      glossaryManager,
      luxOverlay,
      nextActionGenerator,
      progressPoller,
      promptBuilder,
      sessionEndDetector,
      sessionState,
      specEvaluator,
      stagnationDetection,
      stopEvaluator,
      ticketLoader,
      types,
    ]

    expect(modules).toHaveLength(20)
    expect(modules.every(Boolean)).toBe(true)
  })
})
