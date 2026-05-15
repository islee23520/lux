import { afterEach, vi } from 'vitest'

const childProcessMock = {
  execFileSync: vi.fn(() => ''),
  execFile: vi.fn((_cmd, _args, opts) => {
    if (opts?.callback) opts.callback(null, '', '')
    return { on: vi.fn(), kill: vi.fn(), pid: 12345 } as any
  }),
  spawn: vi.fn(() => ({
    stdout: { on: vi.fn() },
    stderr: { on: vi.fn() },
    on: vi.fn(),
    kill: vi.fn(),
    pid: 54321,
    unref: vi.fn(),
  })),
}

;(globalThis as any).require ??= (specifier: string) => {
  if (specifier === 'node:child_process') return childProcessMock
  throw new Error(`Unexpected require: ${specifier}`)
}

vi.mock('node:child_process', () => childProcessMock)

afterEach(() => {
  vi.restoreAllMocks()
  vi.clearAllTimers()
})
