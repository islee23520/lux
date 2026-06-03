export const BRIDGE_TYPE = 'threejs-bridge';
export var CommandType;
(function (CommandType) {
    CommandType["Ping"] = "ping";
    CommandType["Status"] = "status";
    CommandType["Heartbeat"] = "heartbeat";
})(CommandType || (CommandType = {}));
export class InvalidJsonError extends Error {
    constructor() {
        super('invalid json');
        this.name = 'InvalidJsonError';
    }
}
export function parseCommandLine(line) {
    try {
        const parsed = JSON.parse(line);
        if (parsed.command === CommandType.Ping) {
            return { command: CommandType.Ping };
        }
        if (parsed.command === CommandType.Status) {
            return { command: CommandType.Status };
        }
        if (parsed.command === CommandType.Heartbeat) {
            return { command: CommandType.Heartbeat };
        }
        return { command: parsed.command };
    }
    catch {
        throw new InvalidJsonError();
    }
}
export function serializeMessage(message) {
    return `${JSON.stringify(message)}\n`;
}
export function errorResponse(message) {
    return { status: 'error', message };
}
export function pingResponse() {
    return { status: 'ok', type: BRIDGE_TYPE };
}
export function heartbeatAckResponse() {
    return { status: 'ok', type: 'heartbeat_ack' };
}
