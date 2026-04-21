import type { ApiResponse } from './types';

const DEFAULT_PORT = 6982;
const BASE_URL = `http://localhost:${DEFAULT_PORT}`;

export class ApiError extends Error {
	constructor(
		public status: number,
		message: string
	) {
		super(message);
		this.name = 'ApiError';
	}
}

export async function api<T>(path: string, options?: RequestInit): Promise<T> {
	const res = await fetch(`${BASE_URL}${path}`, {
		...options,
		headers: {
			'Content-Type': 'application/json',
			...options?.headers,
		},
	});

	const envelope: ApiResponse<T> = await res.json();

	if (!envelope.success) {
		throw new ApiError(res.status, envelope.error ?? 'Unknown error');
	}

	return envelope.data as T;
}
