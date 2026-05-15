// @ts-nocheck

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest'
import { mkdtempSync, writeFileSync, chmodSync, mkdirSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import { fakeSpawnChild } from './helpers/fake-child-process'

const fetchMock = vi.fn()
globalThis.fetch = fetchMock

const originalPath = process.env.PATH ?? ''

function makeFakeLux(executablePath: string) {
  writeFileSync(
    executablePath,
    '#!/usr/bin/env bash\n# fake lux binary for tests\nsleep 0.05\nexit 0\n',
  )
  chmodSync(executablePath, 0o755)
}

async function loadSubject() {
  vi.resetModules()
  return await import('../gateway-spawn')
}

describe('gateway-spawn', () => {
  let config: {
    gatewayUrl: string
    projectPath: string
    healthTimeoutMs: number
    healthIntervalMs: number
  }

  let tempDir = ''
  let projectDir = ''

  beforeEach(() => {
    vi.useFakeTimers()
    fetchMock.mockReset()

    tempDir = mkdtempSync(join(tmpdir(), 'lux-gateway-test-'))
    projectDir = mkdtempSync(join(tmpdir(), 'lux-project-test-'))
    makeFakeLux(join(tempDir, 'lux'))
    process.env.PATH = `${tempDir}:${originalPath}`

    mkdirSync(projectDir, { recursive: true })
    config = {
      gatewayUrl: 'http://localhost:17340',
      projectPath: projectDir,
      healthTimeoutMs: 1000,
      healthIntervalMs: 100,
    }
  })

  afterEach(() => {
    vi.useRealTimers()
    process.env.PATH = originalPath
  })

  it('should return immediately if gateway is already running', async () => {
    fetchMock.mockResolvedValue({ ok: true })
    const { ensureGatewayRunning } = await loadSubject()

    const result = await ensureGatewayRunning(config)

    expect(result.spawned).toBe(false)
    expect(fetchMock).toHaveBeenCalledWith('http://localhost:17340/api/health', expect.any(Object))
  })

  it('should spawn gateway if not running and wait for health check', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false })
    fetchMock.mockResolvedValueOnce({ ok: true })
    const { ensureGatewayRunning } = await loadSubject()

    const promise = ensureGatewayRunning(config)

    await vi.runAllTimersAsync()

    const result = await promise

    expect(result.spawned).toBe(true)
    expect(typeof result.pid).toBe('number')
    expect(result.readyMs).toBeGreaterThanOrEqual(0)
  })

  it('should timeout if gateway never becomes ready', async () => {
    fetchMock.mockResolvedValue({ ok: false })
    const { ensureGatewayRunning } = await loadSubject()

    const promise = ensureGatewayRunning({
      ...config,
      healthTimeoutMs: 200,
    })

    await vi.runAllTimersAsync()

    const result = await promise
    expect(result.spawned).toBe(true)
    expect(typeof result.pid).toBe('number')
    expect(result.readyMs).toBeGreaterThanOrEqual(0)
  })

  it('should normalize positive fallback values', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false })
    fetchMock.mockResolvedValueOnce({ ok: true })
    const { ensureGatewayRunning } = await loadSubject()

    const promise = ensureGatewayRunning({
      ...config,
      healthTimeoutMs: -1,
      healthIntervalMs: 0,
    })

    await vi.runAllTimersAsync()
    const result = await promise

    expect(result.spawned).toBe(true)
  })

  it('should use default binary lookup when command is absent', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false })
    fetchMock.mockResolvedValueOnce({ ok: true })
    const { ensureGatewayRunning } = await loadSubject()

    const promise = ensureGatewayRunning({ ...config, spawnCommand: undefined })
    await vi.runAllTimersAsync()
    const result = await promise

    expect(result.spawned).toBe(true)
  })

  it('should use custom spawn command and args', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false })
    fetchMock.mockResolvedValueOnce({ ok: true })
    const { ensureGatewayRunning } = await loadSubject()

    const customLux = join(tempDir, 'custom-lux')
    makeFakeLux(customLux)

    const promise = ensureGatewayRunning({
      ...config,
      spawnCommand: customLux,
      spawnArgs: ['serve', '--port', '1234'],
    })

    await vi.runAllTimersAsync()

    const result = await promise
    expect(result.spawned).toBe(true)
    expect(typeof result.pid).toBe('number')
    expect(result.readyMs).toBeGreaterThanOrEqual(0)
  })
})
