import { describe, expect, it, vi } from 'vitest';

import { apiFetch, ApiError } from '@/lib/transport/api';

import { resolveNamespaceTarget } from './resolve-namespace';

vi.mock('@/lib/transport/api', async () => {
	const actual = await vi.importActual<typeof import('@/lib/transport/api')>('@/lib/transport/api');
	return { ...actual, apiFetch: vi.fn() };
});

const mockApiFetch = vi.mocked(apiFetch);

function notFound(): Promise<never> {
	return Promise.reject(new ApiError('not found', 404));
}

describe('resolveNamespaceTarget', () => {
	it('never touches the network for an ordinary one-word query', async () => {
		expect(await resolveNamespaceTarget('minecraft')).toBeNull();
		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('never touches the network for an ordinary multi-word query', async () => {
		expect(await resolveNamespaceTarget('minecraft server')).toBeNull();
		expect(mockApiFetch).not.toHaveBeenCalled();
	});

	it('resolves to the collection when the collection endpoint answers', async () => {
		mockApiFetch.mockImplementation((path) =>
			path.startsWith('/v0/collection/') ? Promise.resolve({}) : notFound()
		);

		const target = await resolveNamespaceTarget('github.com/rabbyte/game-servers');
		expect(target).toEqual({ kind: 'collection', namespace: 'github.com/rabbyte/game-servers' });
	});

	it('resolves to the arrow when only the arrow endpoint answers', async () => {
		mockApiFetch.mockImplementation((path) => (path.startsWith('/v0/arrow/') ? Promise.resolve({}) : notFound()));

		const target = await resolveNamespaceTarget('github.com/rabbyte/minecraft@v1.21.4');
		expect(target).toEqual({ kind: 'arrow', namespace: 'github.com/rabbyte/minecraft@v1.21.4' });
	});

	it('prefers the collection when a namespace happens to answer as both', async () => {
		mockApiFetch.mockResolvedValue({});

		const target = await resolveNamespaceTarget('github.com/rabbyte/both');
		expect(target).toEqual({ kind: 'collection', namespace: 'github.com/rabbyte/both' });
	});

	it('resolves to nothing when neither endpoint knows the namespace', async () => {
		mockApiFetch.mockImplementation(() => notFound());

		expect(await resolveNamespaceTarget('github.com/rabbyte/unknown')).toBeNull();
	});

	it('trims surrounding whitespace before checking the shape and querying', async () => {
		mockApiFetch.mockImplementation((path) =>
			path.startsWith('/v0/collection/') ? Promise.resolve({}) : notFound()
		);

		const target = await resolveNamespaceTarget('  github.com/rabbyte/game-servers  ');
		expect(target).toEqual({ kind: 'collection', namespace: 'github.com/rabbyte/game-servers' });
	});

	it('lets a non-404 failure propagate rather than treating it as "not found"', async () => {
		mockApiFetch.mockRejectedValue(new ApiError('boom', 500));

		await expect(resolveNamespaceTarget('github.com/rabbyte/game-servers')).rejects.toThrow('boom');
	});

	// Collections have no `@ref` in their domain model at all -- quiver.core
	// answers a versioned namespace on /v0/collection with a 500, not a clean
	// 404, since there is no such thing as "not found" for a shape that can
	// never exist. Querying it anyway would make every versioned arrow lookup
	// fail via the propagation above, even though the arrow endpoint itself
	// answers fine.
	it('skips the collection check for a versioned namespace and still resolves the arrow', async () => {
		mockApiFetch.mockImplementation((path) => {
			if (path.startsWith('/v0/collection/')) return Promise.reject(new ApiError('internal error', 500));
			return Promise.resolve({});
		});

		const target = await resolveNamespaceTarget('github.com/char2cs/crowbar@nightly');
		expect(target).toEqual({ kind: 'arrow', namespace: 'github.com/char2cs/crowbar@nightly' });
	});
});
