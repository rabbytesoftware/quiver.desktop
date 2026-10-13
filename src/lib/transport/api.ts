import { backend } from './backend';

export { apiBase } from './backend';

const PROXY_ERROR_HEADER = 'x-quiver-proxy';

interface ApiEnvelope<T> {
	success: boolean;
	error: string | null;
	data?: T;
}

export class ApiError extends Error {
	readonly status: number;

	constructor(message: string, status: number) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
	}
}

export function isNotFoundError(err: unknown): boolean {
	return err instanceof ApiError && err.status === 404;
}

export interface RetryConfig {
	attempts: number;
	baseDelayMs: number;
	maxDelayMs: number;
	sleep?: (ms: number) => Promise<void>;
}

const DEFAULT_RETRY: RetryConfig = { attempts: 8, baseDelayMs: 100, maxDelayMs: 1000 };

const defaultSleep = (ms: number): Promise<void> => new Promise((r) => setTimeout(r, ms));

function isIdempotentRead(init?: RequestInit): boolean {
	const method = init?.method?.toUpperCase();
	return method === undefined || method === 'GET';
}

function isProxyFailure(res: Response): boolean {
	return (res.status === 502 || res.status === 504) && res.headers.get(PROXY_ERROR_HEADER) === 'error';
}

export async function coreIsReachable(): Promise<boolean> {
	const pending = backend().fetch('/v0/health');
	try {
		return (await pending).ok;
	} catch {
		return false;
	}
}

export interface ApiResult<T> {
	status: number;
	data: T;
}

/**
 * `apiFetch` with the response status kept beside the data, for the calls
 * whose success statuses mean different things -- `POST /v0/runtime/:ns/update`
 * answers 202 when an update started and 200 when there was nothing newer.
 */
export async function apiRequest<T>(
	path: string,
	init?: RequestInit,
	retry: RetryConfig = DEFAULT_RETRY
): Promise<ApiResult<T>> {
	const maxAttempts = isIdempotentRead(init) ? Math.max(1, retry.attempts) : 1;
	const sleep = retry.sleep ?? defaultSleep;

	for (let attempt = 1; ; attempt++) {
		const res = await backend().fetch(path, init);

		if (isProxyFailure(res) && attempt < maxAttempts) {
			await sleep(Math.min(retry.maxDelayMs, retry.baseDelayMs * 2 ** (attempt - 1)));
			continue;
		}

		if (isProxyFailure(res)) {
			throw new ApiError(await res.text(), res.status);
		}

		const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;

		if (res.ok && (res.status === 204 || body === null)) {
			return { status: res.status, data: undefined as T };
		}

		// A 202 may carry an envelope -- POST /v0/search/discover answers with the
		// job ticket before any provider has been asked, and the caller needs it.
		// A 202 carrying anything else is a bare acknowledgement, as before.
		if (res.status === 202 && !body?.success) {
			return { status: res.status, data: undefined as T };
		}

		if (!res.ok || !body?.success) {
			throw new ApiError(body?.error ?? `${res.status} ${res.statusText}`, res.status);
		}

		return { status: res.status, data: body.data as T };
	}
}

export async function apiFetch<T>(path: string, init?: RequestInit, retry: RetryConfig = DEFAULT_RETRY): Promise<T> {
	return (await apiRequest<T>(path, init, retry)).data;
}
