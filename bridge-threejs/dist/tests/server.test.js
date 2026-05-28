import net from 'node:net';
import { afterEach, describe, expect, it } from 'vitest';
import { createBridgeServer } from '../src/server.js';
function waitForEvent(register, timeoutMs = 2_000) {
    return new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error(`Timed out after ${timeoutMs}ms`)), timeoutMs);
        register((value) => {
            clearTimeout(timer);
            resolve(value);
        });
    });
}
function readJsonLine(socket, timeoutMs = 2_000) {
    return waitForEvent((resolve) => {
        let buffer = '';
        const onData = (chunk) => {
            buffer += chunk.toString('utf8');
            const newlineIndex = buffer.indexOf('\n');
            if (newlineIndex === -1) {
                return;
            }
            const line = buffer.slice(0, newlineIndex).trim();
            socket.off('data', onData);
            resolve(JSON.parse(line));
        };
        socket.on('data', onData);
    }, timeoutMs);
}
function connectClient(port) {
    return new Promise((resolve, reject) => {
        const socket = net.createConnection({ host: '127.0.0.1', port });
        socket.once('connect', () => resolve(socket));
        socket.once('error', reject);
    });
}
function writeJson(socket, payload) {
    socket.write(`${JSON.stringify(payload)}\n`);
}
const serversToClose = [];
const socketsToClose = [];
afterEach(async () => {
    for (const socket of socketsToClose.splice(0)) {
        socket.destroy();
    }
    for (const server of serversToClose.splice(0)) {
        await server.stop();
    }
});
describe('Three.js bridge TCP server', () => {
    it('accepts a TCP connection and replies to ping with line-delimited JSON', async () => {
        const server = createBridgeServer({ port: 0, heartbeatIntervalMs: 30_000, cdpEndpointUrl: 'http://localhost:9222' });
        serversToClose.push(server);
        await server.start();
        const socket = await connectClient(server.port);
        socketsToClose.push(socket);
        writeJson(socket, { command: 'ping' });
        const message = await readJsonLine(socket);
        expect(message).toEqual({ status: 'ok', type: 'threejs-bridge' });
    });
    it('returns bridge status for the status command', async () => {
        const server = createBridgeServer({ port: 0, heartbeatIntervalMs: 30_000, cdpEndpointUrl: 'http://localhost:9222' });
        serversToClose.push(server);
        await server.start();
        const socket = await connectClient(server.port);
        socketsToClose.push(socket);
        writeJson(socket, { command: 'status' });
        const message = await readJsonLine(socket);
        expect(message).toMatchObject({
            status: 'ok',
            type: 'threejs-bridge',
            port: server.port,
            heartbeatIntervalMs: 30_000,
            cdpEndpointUrl: 'http://localhost:9222'
        });
    });
    it('returns an error response for invalid JSON without crashing the server', async () => {
        const server = createBridgeServer({ port: 0, heartbeatIntervalMs: 30_000, cdpEndpointUrl: 'http://localhost:9222' });
        serversToClose.push(server);
        await server.start();
        const socket = await connectClient(server.port);
        socketsToClose.push(socket);
        socket.write('{"command":"ping"\n');
        const invalidResponse = await readJsonLine(socket);
        expect(invalidResponse).toEqual({ status: 'error', message: 'invalid json' });
        writeJson(socket, { command: 'ping' });
        const followupResponse = await readJsonLine(socket);
        expect(followupResponse).toEqual({ status: 'ok', type: 'threejs-bridge' });
    });
    it('sends heartbeat requests on persistent connections and accepts heartbeat responses', async () => {
        const server = createBridgeServer({ port: 0, heartbeatIntervalMs: 25, cdpEndpointUrl: 'http://localhost:9222' });
        serversToClose.push(server);
        await server.start();
        const socket = await connectClient(server.port);
        socketsToClose.push(socket);
        const heartbeatRequest = await readJsonLine(socket, 2_000);
        expect(heartbeatRequest).toEqual({ command: 'heartbeat' });
        writeJson(socket, { command: 'heartbeat' });
        const heartbeatResponse = await readJsonLine(socket, 2_000);
        expect(heartbeatResponse).toEqual({ status: 'ok', type: 'heartbeat_ack' });
    });
    it('supports multiple simultaneous connections', async () => {
        const server = createBridgeServer({ port: 0, heartbeatIntervalMs: 30_000, cdpEndpointUrl: 'http://localhost:9222' });
        serversToClose.push(server);
        await server.start();
        const first = await connectClient(server.port);
        const second = await connectClient(server.port);
        socketsToClose.push(first, second);
        writeJson(first, { command: 'ping' });
        writeJson(second, { command: 'ping' });
        await expect(readJsonLine(first)).resolves.toEqual({ status: 'ok', type: 'threejs-bridge' });
        await expect(readJsonLine(second)).resolves.toEqual({ status: 'ok', type: 'threejs-bridge' });
    });
});
