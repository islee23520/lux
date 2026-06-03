import { loadConfig } from './config.js';
import { createBridgeServer } from './server.js';
async function main() {
    const config = loadConfig();
    const server = createBridgeServer(config);
    await server.start();
    process.stdout.write(`[lux-threejs-bridge] listening on 127.0.0.1:${server.port} heartbeat=${server.heartbeatIntervalMs}ms cdp=${server.cdpEndpointUrl}\n`);
    const shutdown = async (signal) => {
        process.stdout.write(`[lux-threejs-bridge] shutting down on ${signal}\n`);
        await server.stop();
        process.exit(0);
    };
    process.once('SIGINT', () => {
        void shutdown('SIGINT');
    });
    process.once('SIGTERM', () => {
        void shutdown('SIGTERM');
    });
}
main().catch((error) => {
    const message = error instanceof Error ? error.stack ?? error.message : String(error);
    process.stderr.write(`${message}\n`);
    process.exit(1);
});
