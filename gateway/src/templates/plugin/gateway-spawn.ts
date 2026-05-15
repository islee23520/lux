// gateway-spawn.ts — Auto-spawn Lux gateway when plugin loads
// Ensures lux serve is running before poller/orchestrator start.

declare const process: { env: Record<string, string | undefined> }

export interface GatewaySpawnConfig {
  gatewayUrl: string
  projectPath: string
  healthTimeoutMs?: number
  healthIntervalMs?: number
  spawnCommand?: string
  spawnArgs?: string[]
}

export interface GatewaySpawnResult {
  spawned: boolean
  pid?: number
  readyMs: number
}

const DEFAULT_HEALTH_TIMEOUT_MS = 15_000
const DEFAULT_HEALTH_INTERVAL_MS = 500
const HEALTH_CHECK_TIMEOUT_MS = 3_000

type SpawnedProcess = {
  pid?: number
  unref: () => void
}

type SpawnFunction = (
  command: string,
  args: string[],
  options: {
    detached: boolean
    stdio: "ignore"
    cwd: string
    env: Record<string, string | undefined>
  },
) => SpawnedProcess

/**
 * Check if gateway is reachable.
 */
async function checkHealth(gatewayUrl: string): Promise<boolean> {
  try {
    const url = `${gatewayUrl.replace(/\/+$/, "")}/api/health`
    const res = await fetch(url, { signal: AbortSignal.timeout(HEALTH_CHECK_TIMEOUT_MS) })
    return res.ok
  } catch {
    return false
  }
}

/**
 * Resolve the path to the `lux` binary.
 * Checks PATH first, then falls back to common locations.
 */
function resolveLuxBinary(): string | null {
  const candidates = [
    "lux",
    "/Users/ilseoblee/.cargo/bin/lux",
  ]
  return candidates[0] ?? null
}

function normalizePositiveNumber(value: number | undefined, fallback: number): number {
  return value && Number.isFinite(value) && value > 0 ? value : fallback
}

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms))
}

/**
 * Ensure the Lux gateway is running. Spawn it if not.
 *
 * This is the main export — call it during plugin initialization,
 * BEFORE starting the progress poller or orchestrator.
 */
export async function ensureGatewayRunning(config: GatewaySpawnConfig): Promise<GatewaySpawnResult> {
  const startTime = Date.now()
  const timeout = normalizePositiveNumber(config.healthTimeoutMs, DEFAULT_HEALTH_TIMEOUT_MS)
  const interval = normalizePositiveNumber(config.healthIntervalMs, DEFAULT_HEALTH_INTERVAL_MS)
  const gatewayUrl = config.gatewayUrl.replace(/\/+$/, "")

  const alreadyRunning = await checkHealth(gatewayUrl)
  if (alreadyRunning) {
    console.debug("[Lux] Gateway already running at", gatewayUrl)
    return { spawned: false, readyMs: Date.now() - startTime }
  }

  const command = config.spawnCommand ?? resolveLuxBinary() ?? "lux"
  const args = config.spawnArgs ?? ["serve"]
  console.info("[Lux] Gateway not reachable; spawning", { gatewayUrl, command, args })

  let pid: number | undefined

  try {
    const { spawn } = require("node:child_process")
    const child = spawn(command, args, {
      detached: true,
      stdio: "ignore",
      cwd: config.projectPath,
      env: { ...process.env },
    })

    pid = child.pid
    child.unref()
    console.debug("[Lux] Gateway spawned", { pid })
  } catch (spawnErr) {
    console.warn("[Lux] Failed to spawn gateway via child_process:", spawnErr)
  }

  const deadline = startTime + timeout
  while (Date.now() < deadline) {
    await sleep(interval)
    if (await checkHealth(gatewayUrl)) {
      const readyMs = Date.now() - startTime
      console.debug("[Lux] Gateway ready", { readyMs })
      return { spawned: true, pid, readyMs }
    }
  }

  console.warn(
    "[Lux] Gateway did not become ready within",
    timeout,
    "ms. Plugin will operate in degraded mode (polling errors expected). Start manually: lux serve",
  )
  return { spawned: true, pid, readyMs: Date.now() - startTime }
}
