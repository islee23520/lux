import { vi } from 'vitest'

type FakeSpawnOptions = {
  stdout?: string
  stderr?: string
  exitCode?: number
  delayMs?: number
}

type FakeExecFileSyncOptions = {
  stdout?: string
  stderr?: string
  exitCode?: number
}

type Listener = (...args: unknown[]) => void

class MiniEmitter {
  private listeners = new Map<string, Listener[]>()

  on(event: string, listener: Listener): this {
    const current = this.listeners.get(event) ?? []
    current.push(listener)
    this.listeners.set(event, current)
    return this
  }

  once(event: string, listener: Listener): this {
    const wrapped: Listener = (...args) => {
      this.off(event, wrapped)
      listener(...args)
    }
    return this.on(event, wrapped)
  }

  off(event: string, listener: Listener): this {
    const current = this.listeners.get(event) ?? []
    this.listeners.set(event, current.filter((item) => item !== listener))
    return this
  }

  emit(event: string, ...args: unknown[]): boolean {
    const current = this.listeners.get(event) ?? []
    for (const listener of current) listener(...args)
    return current.length > 0
  }
}

type FakeChildProcess = MiniEmitter & {
  stdout: MiniEmitter
  stderr: MiniEmitter
  stdin: MiniEmitter
  pid: number
  killed: boolean
  kill: () => boolean
  ref: () => FakeChildProcess
  unref: () => FakeChildProcess
  on: (event: string, listener: Listener) => FakeChildProcess
  once: (event: string, listener: Listener) => FakeChildProcess
}

function createStreamEmitter() {
  return new MiniEmitter()
}

export function fakeSpawnChild(options: FakeSpawnOptions = {}) {
  const child = new MiniEmitter() as FakeChildProcess

  child.stdout = createStreamEmitter()
  child.stderr = createStreamEmitter()
  child.stdin = createStreamEmitter()
  child.pid = 54321
  child.killed = false

  child.on = vi.fn((event: string, listener: Listener) => {
    MiniEmitter.prototype.on.call(child, event, listener)
    return child
  }) as FakeChildProcess['on']
  child.once = vi.fn((event: string, listener: Listener) => {
    MiniEmitter.prototype.once.call(child, event, listener)
    return child
  }) as FakeChildProcess['once']
  child.kill = vi.fn(() => {
    child.killed = true
    queueMicrotask(() => {
      child.emit('close', options.exitCode ?? 0, null)
      child.emit('exit', options.exitCode ?? 0, null)
    })
    return true
  }) as FakeChildProcess['kill']
  child.ref = vi.fn(() => child) as FakeChildProcess['ref']
  child.unref = vi.fn(() => child) as FakeChildProcess['unref']

  queueMicrotask(() => {
    if (options.stdout) child.stdout.emit('data', options.stdout)
    if (options.stderr) child.stderr.emit('data', options.stderr)
    setTimeout(() => {
      child.emit('close', options.exitCode ?? 0, null)
      child.emit('exit', options.exitCode ?? 0, null)
    }, options.delayMs ?? 5)
  })

  return child
}

export function fakeExecFileSyncResult(options: FakeExecFileSyncOptions = {}) {
  const stdout = options.stdout ?? ''
  const stderr = options.stderr ?? ''
  const exitCode = options.exitCode ?? 0

  return vi.fn(() => {
    if (exitCode !== 0) {
      const error = new Error(stderr || 'command failed') as Error & {
        status?: number
        stdout?: string
        stderr?: string
      }
      error.status = exitCode
      error.stdout = stdout
      error.stderr = stderr
      throw error
    }

    return stdout
  })
}
