// @ts-nocheck
import fs from 'node:fs'
import os from 'node:os'
import path from 'node:path'
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { checkAndFixCompile, runCompile, parseCompileErrors, buildFixPrompt } from '../compile-guard'
import { CLEAN_COMPILE, CS_ERROR_OUTPUT, MULTI_ERROR_OUTPUT } from './fixtures/compile-output'

describe('compile-guard', () => {
  let originalPath = process.env.PATH ?? ''
  let tempDir: string | undefined

  const config = {
    projectPath: '/test/project',
    gatewayUrl: 'http://localhost:17340',
    sessionID: 'test-session'
  }

  const ctx = {
    directory: '/test/project',
    client: {
      session: {
        promptAsync: vi.fn().mockResolvedValue({})
      }
    }
  }

  function setLuxBinary(stdout: string, stderr = '', exitCode = 0) {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'lux-compile-guard-'))
    const luxPath = path.join(tempDir, 'lux')
    const stdoutB64 = Buffer.from(stdout, 'utf8').toString('base64')
    const stderrB64 = Buffer.from(stderr, 'utf8').toString('base64')
    const script = exitCode === 0
      ? `#!/bin/sh
node -e "process.stdout.write(Buffer.from(process.argv[1], 'base64').toString('utf8'))" ${stdoutB64}
`
      : `#!/bin/sh
node -e "process.stderr.write(Buffer.from(process.argv[1], 'base64').toString('utf8')); process.exit(${exitCode})" ${stderrB64}
`
    fs.writeFileSync(luxPath, script, { mode: 0o755 })
    fs.chmodSync(luxPath, 0o755)
    process.env.PATH = `${tempDir}${path.delimiter}${originalPath}`
  }

  beforeEach(() => {
    ctx.client.session.promptAsync.mockClear()
  })

  afterEach(() => {
    process.env.PATH = originalPath
    if (tempDir) {
      fs.rmSync(tempDir, { recursive: true, force: true })
      tempDir = undefined
    }
  })

  describe('runCompile', () => {
    it('should return success output when compile succeeds', () => {
      setLuxBinary(CLEAN_COMPILE.stdout)
      
      const result = runCompile(config.projectPath)
      
      expect(result.success).toBe(true)
      expect(result.stdout).toBe(CLEAN_COMPILE.stdout)
    })

    it('should return failure output when compile fails', () => {
      setLuxBinary('', CS_ERROR_OUTPUT.stderr, 1)

      const result = runCompile(config.projectPath)

      expect(result.success).toBe(false)
      expect(result.exitCode).toBe(1)
      expect(result.stderr).toBe(CS_ERROR_OUTPUT.stderr)
    })

    it('should preserve custom object stdout from process buffers', () => {
      const originalExecFileSync = (runCompile as unknown as { __proto__?: never })
      void originalExecFileSync
      const childProcessModule = require('node:child_process') as { execFileSync: ReturnType<typeof vi.fn> }
      const spy = vi.spyOn(childProcessModule, 'execFileSync').mockImplementation(() => {
        throw { stdout: { toString: () => 'custom-output' }, stderr: undefined, status: 1 }
      })

      const result = runCompile(config.projectPath)

      expect(result.stdout).toBe('custom-output')
      spy.mockRestore()
    })
  })

  describe('parseCompileErrors', () => {
    it('should parse standard C# errors', () => {
      const errors = parseCompileErrors(CS_ERROR_OUTPUT as any)
      
      expect(errors).toHaveLength(2)
      expect(errors[0]).toMatchObject({
        file: 'Assets/Scripts/PlayerController.cs',
        line: 42,
        column: 18,
        severity: 'error',
        code: 'CS0103'
      })
      expect(errors[1].severity).toBe('warning')
    })

    it('should handle multiple errors across files', () => {
      const errors = parseCompileErrors(MULTI_ERROR_OUTPUT as any)
      expect(errors).toHaveLength(3)
      expect(errors.filter(e => e.severity === 'error')).toHaveLength(2)
    })

    it('should parse bracketed diagnostics and ignore duplicates', () => {
      const errors = parseCompileErrors({
        stdout: [
          'Assets/Scripts/Foo.cs[12,3]: error CS1001: Missing semicolon',
          'Assets/Scripts/Foo.cs[12,3]: error CS1001: Missing semicolon',
          'Assets/Scripts/Bar.cs[4,1]: warning UCE2002: Be careful',
        ].join('\n'),
        stderr: '',
        exitCode: 1,
        success: false,
        durationMs: 1,
      } as any)

      expect(errors).toHaveLength(2)
      expect(errors[0]).toMatchObject({
        file: 'Assets/Scripts/Foo.cs',
        line: 12,
        column: 3,
        severity: 'error',
        code: 'CS1001',
      })
      expect(errors[1]).toMatchObject({
        file: 'Assets/Scripts/Bar.cs',
        line: 4,
        column: 1,
        severity: 'warning',
        code: 'UCE2002',
      })
    })
  })

  describe('checkAndFixCompile', () => {
    it('should return success if no errors found', async () => {
      setLuxBinary(CLEAN_COMPILE.stdout)
      const state = { consecutiveCompileFailures: 0 }

      const result = await checkAndFixCompile(ctx as any, config, state)

      expect(result.hasErrors).toBe(false)
      expect(ctx.client.session.promptAsync).not.toHaveBeenCalled()
    })

    it('should trigger fix prompt if errors found', async () => {
      setLuxBinary('', CS_ERROR_OUTPUT.stderr, 1)
      
      const state = { consecutiveCompileFailures: 0 }

      const result = await checkAndFixCompile(ctx as any, config, state)

      expect(result.hasErrors).toBe(true)
      expect(result.wasFixed).toBe(true)
      expect(ctx.client.session.promptAsync).toHaveBeenCalled()
      expect(state.consecutiveCompileFailures).toBe(1)
    })

    it('should warn and retry when promptAsync is missing', async () => {
      setLuxBinary('', CS_ERROR_OUTPUT.stderr, 1)
      const warnSpy = vi.spyOn(console, 'warn').mockImplementation(() => {})
      const state = { consecutiveCompileFailures: 0 }
      const noPromptCtx = { directory: '/test/project', client: { session: {} } }

      const result = await checkAndFixCompile(noPromptCtx as any, config, state)

      expect(result.hasErrors).toBe(true)
      expect(result.shouldRetry).toBe(true)
      expect(warnSpy).toHaveBeenCalledWith(
        '[Lux] Compile auto-fix unavailable: promptAsync missing',
        { projectPath: config.projectPath },
      )
      warnSpy.mockRestore()
    })

    it('should not retry if max retries exhausted', async () => {
      setLuxBinary('', CS_ERROR_OUTPUT.stderr, 1)
      
      const state = { consecutiveCompileFailures: 3 }

      const result = await checkAndFixCompile(ctx as any, config, state)

      expect(result.wasFixed).toBe(false)
      expect(ctx.client.session.promptAsync).not.toHaveBeenCalled()
    })
  })

  describe('buildFixPrompt', () => {
    it('should include error details in prompt', () => {
      const errors = parseCompileErrors(CS_ERROR_OUTPUT as any)
      const prompt = buildFixPrompt(errors.filter(e => e.severity === 'error'))
      
      expect(prompt).toContain('CS0103')
      expect(prompt).toContain('PlayerController.cs:42:18')
      expect(prompt).toContain('Fix ALL of the following')
    })
  })
})
