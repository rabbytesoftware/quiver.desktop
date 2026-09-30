import http from 'node:http';

import { socketPath } from './paths';

/**
 * A direct line to the quiver.core daemon the app under test spawned --
 * deliberately NOT through the app's Tauri proxy.
 *
 * The point of these assertions is that core really holds the rows, so they
 * must not be answered by anything inside the window they are meant to
 * corroborate. That means speaking HTTP over core's own local endpoint: the unix
 * socket (`{QUIVER_HOME}/quiver.sock`), or on Windows the named pipe
 * (`\\.\pipe\quiver`), which node's `socketPath` accepts unchanged.
 */
export interface CoreClient {
	get<T>(path: string): Promise<{ status: number; body: T | null }>;
	post<T>(path: string, body?: unknown): Promise<{ status: number; body: T | null }>;
}

/**
 * Every v0 response is wrapped. `libs.WriteQueryOK` / `WriteMutationOK` emit
 * `{success, error, data}`, and the app's own `apiFetch`
 * (src/lib/transport/api.ts) returns `body.data` after checking `body.success`.
 * This client unwraps the same way, so a spec asserting on `body.user_installed`
 * is asserting on the same field the app itself would read.
 */
interface Envelope<T> {
	success: boolean;
	error: string | null;
	data?: T;
}

/** `GET /v0/arrow/:ns` -- mirrors src/lib/core-store/dtos/v0/arrow.ts's ArrowDetailDTO. */
export interface ArrowDetailDTO {
	/** The catalog identity, `namespace@selector`, always. */
	namespace: string;
	name: string;
	description: string;
	state: string;
	tags: string[] | null;
	installed_at?: string;
	user_installed: boolean;
	selector_kind?: string;
	resolved_ref?: string;
	installed_commit?: string;
	/**
	 * The row's own record of what is ahead, set by the version check on the
	 * way through `GetDetail`. SEPARATE from `state`: the runtime's `outdated`
	 * state is reconciled from it, and only from `ready` -- `absent ->
	 * outdated` is not a legal transition.
	 */
	available?: { ref: string; commit: string };
	outdated?: boolean;
	active_run?: { method: string; pid?: number; variables: Record<string, string> } | null;
	last_return?: { method: string; outcome: string } | null;
}

function request<T>(
	home: string,
	method: string,
	reqPath: string,
	payload?: unknown
): Promise<{ status: number; body: T | null }> {
	const data = payload === undefined ? undefined : JSON.stringify(payload);

	const options: http.RequestOptions = { socketPath: socketPath(home), method, path: reqPath };

	return new Promise((resolve, reject) => {
		const req = http.request(
			{
				...options,
				headers: {
					// A Host header is mandatory for HTTP/1.1; over a unix socket
					// there is no meaningful authority, and gin does not care what
					// it says -- only that it is present.
					Host: 'localhost',
					...(data ? { 'Content-Type': 'application/json', 'Content-Length': Buffer.byteLength(data) } : {}),
				},
			},
			(res) => {
				const chunks: Buffer[] = [];
				res.on('data', (c: Buffer) => chunks.push(c));
				res.on('end', () => {
					const raw = Buffer.concat(chunks).toString('utf8');
					let body: T | null = null;
					try {
						const parsed = raw ? (JSON.parse(raw) as Envelope<T>) : null;
						// Unwrap the envelope when there is one; fall back to the
						// raw document for the few endpoints that answer plainly.
						body =
							parsed && typeof parsed === 'object' && 'success' in parsed
								? ((parsed.data ?? null) as T | null)
								: ((parsed as unknown as T) ?? null);
					} catch {
						body = null;
					}
					resolve({ status: res.statusCode ?? 0, body });
				});
			}
		);
		req.on('error', reject);
		if (data) req.write(data);
		req.end();
	});
}

export function coreClient(home: string): CoreClient {
	return {
		get: (p) => request(home, 'GET', p),
		post: (p, b) => request(home, 'POST', p, b),
	};
}

/**
 * `GET /v0/arrow/:ns`. The namespace is percent-encoded exactly as the app
 * itself encodes it (`src/lib/core-store/queries/arrow.ts`), so this exercises
 * the same route shape the product uses rather than a second, looser one.
 */
export async function getArrow(
	home: string,
	namespace: string
): Promise<{ status: number; body: ArrowDetailDTO | null }> {
	return coreClient(home).get<ArrowDetailDTO>(`/v0/arrow/${encodeURIComponent(namespace)}`);
}

/** `GET /v0/health` -- the same probe `SidecarManager::ensure_running` decides readiness on. */
export async function healthy(home: string): Promise<boolean> {
	try {
		const { status } = await coreClient(home).get('/v0/health');
		return status === 200;
	} catch {
		return false;
	}
}

/** Blocks until core answers `/v0/health`, or the deadline passes. */
export async function waitForCore(home: string, timeoutMs = 60_000): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		if (await healthy(home)) return;
		await new Promise((r) => setTimeout(r, 250));
	}
	throw new Error(`quiver.core never answered /v0/health at ${socketPath(home)} within ${timeoutMs}ms`);
}

/** Whether an OS process is still alive. Signal 0 tests for existence without delivering anything. */
export function processAlive(pid: number): boolean {
	try {
		process.kill(pid, 0);
		return true;
	} catch (err) {
		return (err as NodeJS.ErrnoException).code === 'EPERM';
	}
}
