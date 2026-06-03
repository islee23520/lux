import net from 'node:net';
import { BRIDGE_TYPE, CommandType, InvalidJsonError, errorResponse, heartbeatAckResponse, parseCommandLine, pingResponse, serializeMessage } from './protocol.js';
export function createBridgeServer(config) {
    const clients = new Set();
    const server = net.createServer();
    let boundPort = config.port;
    let started = false;
    server.on('connection', (socket) => {
        const client = {
            socket,
            buffer: '',
            heartbeatTimer: undefined,
            awaitingHeartbeat: false
        };
        clients.add(client);
        socket.setEncoding('utf8');
        socket.setNoDelay(true);
        socket.on('data', (chunk) => {
            void handleData(client, chunk);
        });
        socket.on('close', () => {
            clearHeartbeat(client);
            clients.delete(client);
        });
        socket.on('error', () => {
            clearHeartbeat(client);
            clients.delete(client);
        });
        scheduleHeartbeat(client);
    });
    return {
        get port() {
            return boundPort;
        },
        get cdpEndpointUrl() {
            return config.cdpEndpointUrl;
        },
        get heartbeatIntervalMs() {
            return config.heartbeatIntervalMs;
        },
        async start() {
            if (started) {
                return;
            }
            await new Promise((resolve, reject) => {
                server.once('error', reject);
                server.listen(config.port, '127.0.0.1', () => {
                    server.off('error', reject);
                    const address = server.address();
                    if (!address || typeof address === 'string') {
                        reject(new Error('Unable to resolve TCP server address'));
                        return;
                    }
                    boundPort = address.port;
                    started = true;
                    resolve();
                });
            });
        },
        async stop() {
            clearAllHeartbeats(clients);
            for (const client of clients) {
                client.socket.destroy();
            }
            clients.clear();
            if (!started) {
                return;
            }
            await new Promise((resolve, reject) => {
                server.close((error) => {
                    if (error) {
                        reject(error);
                        return;
                    }
                    started = false;
                    resolve();
                });
            });
        }
    };
    async function handleData(client, chunk) {
        client.buffer += chunk;
        while (true) {
            const newlineIndex = client.buffer.indexOf('\n');
            if (newlineIndex === -1) {
                return;
            }
            const rawLine = client.buffer.slice(0, newlineIndex);
            client.buffer = client.buffer.slice(newlineIndex + 1);
            const line = rawLine.trim();
            if (line.length === 0) {
                continue;
            }
            try {
                const command = parseCommandLine(line);
                if (command.command === CommandType.Ping) {
                    writeMessage(client.socket, pingResponse());
                    continue;
                }
                if (command.command === CommandType.Status) {
                    writeMessage(client.socket, buildStatusResponse());
                    continue;
                }
                if (command.command === CommandType.Heartbeat) {
                    client.awaitingHeartbeat = false;
                    writeMessage(client.socket, heartbeatAckResponse());
                    continue;
                }
                writeMessage(client.socket, errorResponse(`unknown command: ${String(command.command)}`));
            }
            catch (error) {
                if (error instanceof InvalidJsonError) {
                    writeMessage(client.socket, errorResponse(error.message));
                    continue;
                }
                writeMessage(client.socket, errorResponse('internal server error'));
            }
        }
    }
    function scheduleHeartbeat(client) {
        clearHeartbeat(client);
        client.heartbeatTimer = setInterval(() => {
            if (client.socket.destroyed) {
                clearHeartbeat(client);
                return;
            }
            if (client.awaitingHeartbeat) {
                client.socket.destroy();
                return;
            }
            client.awaitingHeartbeat = true;
            writeMessage(client.socket, { command: CommandType.Heartbeat });
        }, config.heartbeatIntervalMs);
    }
    function buildStatusResponse() {
        return {
            status: 'ok',
            type: BRIDGE_TYPE,
            port: boundPort,
            heartbeatIntervalMs: config.heartbeatIntervalMs,
            cdpEndpointUrl: config.cdpEndpointUrl,
            activeConnections: clients.size
        };
    }
}
function writeMessage(socket, message) {
    socket.write(serializeMessage(message));
}
function clearHeartbeat(client) {
    if (client.heartbeatTimer !== undefined) {
        clearInterval(client.heartbeatTimer);
        client.heartbeatTimer = undefined;
    }
}
function clearAllHeartbeats(clients) {
    for (const client of clients) {
        clearHeartbeat(client);
    }
}
