import { WS_CLOSE_SENTINEL, parseCloseInfo } from '@/lib/transport/quiver-socket';

/** Marks messages exchanged with the shim. */
export const SHIM_TAG = '__arrowShim';

/** Sockets one arrow may hold at once. */
export const MAX_SOCKETS_PER_ARROW = 8;

const MAX_ID_LENGTH = 32;
const ABNORMAL_CLOSE = 1006;
const NORMAL_CLOSE = 1000;

export interface WsApi {
	wsOpen(host: string, id: string, path: string, onFrame: (text: string) => void): Promise<void>;
	wsSend(host: string, id: string, data: string): Promise<void>;
	wsClose(host: string, id: string): Promise<void>;
}

export interface BridgeDeps {
	api: WsApi;
	/** The iframe window currently registered for an arrow host. */
	frameWindow(host: string): Window | undefined;
	hosts(): string[];
}

interface ShimMessage {
	type: string;
	id: string;
	path?: unknown;
	data?: unknown;
}

function isShimMessage(data: unknown): data is ShimMessage {
	if (!data || typeof data !== 'object') return false;
	const m = data as Record<string, unknown>;
	return (
		m[SHIM_TAG] === 1 &&
		typeof m.type === 'string' &&
		typeof m.id === 'string' &&
		m.id.length > 0 &&
		m.id.length <= MAX_ID_LENGTH
	);
}

/**
 * Relays WebSocket frames between arrow pages and the daemon. The only
 * authority is `event.source`: a message is honoured only when it came from
 * the iframe registered for a host, and only touches that host's sockets.
 */
export function createBridge(deps: BridgeDeps) {
	const open = new Map<string, Set<string>>();

	const reply = (host: string, message: Record<string, unknown>) => {
		deps.frameWindow(host)?.postMessage({ [SHIM_TAG]: 1, ...message }, '*');
	};

	const hostOf = (source: MessageEventSource | null) =>
		source ? deps.hosts().find((h) => deps.frameWindow(h) === source) : undefined;

	const track = (host: string) => {
		let ids = open.get(host);
		if (!ids) {
			ids = new Set();
			open.set(host, ids);
		}
		return ids;
	};

	const finish = (host: string, id: string, code: number, reason = '') => {
		if (open.get(host)?.delete(id)) reply(host, { type: 'ws-close', id, code, reason });
	};

	const onFrame = (host: string, id: string) => (text: string) => {
		if (!open.get(host)?.has(id)) return;
		if (!text.startsWith(WS_CLOSE_SENTINEL)) {
			reply(host, { type: 'ws-message', id, data: text });
			return;
		}
		const info = parseCloseInfo(text);
		finish(host, id, info?.code ?? ABNORMAL_CLOSE, info?.reason ?? '');
	};

	async function handleOpen(host: string, m: ShimMessage) {
		if (typeof m.path !== 'string') return;
		const ids = track(host);
		if (ids.has(m.id)) return;
		if (ids.size >= MAX_SOCKETS_PER_ARROW) {
			reply(host, { type: 'ws-close', id: m.id, code: ABNORMAL_CLOSE, reason: 'too many sockets' });
			return;
		}
		ids.add(m.id);
		try {
			await deps.api.wsOpen(host, m.id, m.path, onFrame(host, m.id));
			if (ids.has(m.id)) reply(host, { type: 'ws-open', id: m.id });
			else void deps.api.wsClose(host, m.id).catch(() => {});
		} catch {
			reply(host, { type: 'ws-error', id: m.id });
			finish(host, m.id, ABNORMAL_CLOSE);
		}
	}

	async function onMessage(event: MessageEvent) {
		const host = hostOf(event.source);
		if (!host || !isShimMessage(event.data)) return;
		const m = event.data;

		if (m.type === 'ws-open') {
			await handleOpen(host, m);
		} else if (m.type === 'ws-send' && typeof m.data === 'string' && open.get(host)?.has(m.id)) {
			try {
				await deps.api.wsSend(host, m.id, m.data);
			} catch {
				reply(host, { type: 'ws-error', id: m.id });
			}
		} else if (m.type === 'ws-close' && open.get(host)?.has(m.id)) {
			try {
				await deps.api.wsClose(host, m.id);
			} catch {
				// the socket is gone either way
			} finally {
				finish(host, m.id, NORMAL_CLOSE);
			}
		}
	}

	function closeHost(host: string) {
		const ids = open.get(host);
		if (!ids) return;
		for (const id of ids) void deps.api.wsClose(host, id).catch(() => {});
		ids.clear();
	}

	function dispose() {
		for (const host of [...open.keys()]) closeHost(host);
	}

	return { onMessage, closeHost, dispose };
}
