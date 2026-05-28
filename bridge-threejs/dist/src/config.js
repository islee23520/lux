export const DEFAULT_PORT = 19_876;
export const DEFAULT_HEARTBEAT_INTERVAL_MS = 30_000;
export const DEFAULT_CDP_ENDPOINT_URL = 'http://localhost:9222';
export function loadConfig(options = {}) {
    const args = options.args ?? process.argv.slice(2);
    const env = options.env ?? process.env;
    const cliPort = readCliPort(args);
    const envPort = parseInteger(env.LUX_THREEJS_BRIDGE_PORT);
    const envHeartbeatIntervalMs = parseInteger(env.LUX_THREEJS_BRIDGE_HEARTBEAT_MS);
    return {
        port: cliPort ?? envPort ?? DEFAULT_PORT,
        cdpEndpointUrl: env.LUX_THREEJS_CDP_ENDPOINT_URL ?? DEFAULT_CDP_ENDPOINT_URL,
        heartbeatIntervalMs: envHeartbeatIntervalMs ?? DEFAULT_HEARTBEAT_INTERVAL_MS
    };
}
function readCliPort(args) {
    for (let index = 0; index < args.length; index += 1) {
        const current = args[index];
        if (current === undefined) {
            continue;
        }
        if (current === '--port') {
            return parseRequiredInteger(args[index + 1], '--port');
        }
        if (current.startsWith('--port=')) {
            return parseRequiredInteger(current.slice('--port='.length), '--port');
        }
    }
    return undefined;
}
function parseInteger(value) {
    if (value === undefined || value.trim() === '') {
        return undefined;
    }
    return parseRequiredInteger(value, 'environment variable');
}
function parseRequiredInteger(value, source) {
    if (value === undefined) {
        throw new Error(`Missing numeric value for ${source}`);
    }
    const parsed = Number.parseInt(value, 10);
    if (!Number.isFinite(parsed) || parsed < 0) {
        throw new Error(`Invalid numeric value for ${source}: ${value}`);
    }
    return parsed;
}
