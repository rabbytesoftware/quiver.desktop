import { describe, expect, it, vi } from 'vitest';

import { ApiError } from '@/lib/transport/api';

import { resolveNamespaceTarget } from './resolve-namespace';

vi.mock('@/lib/transport/api', async () => {
	const actual = await vi.importActual<typeof import('@/lib/transport/api')>('@/lib/transport/api');
	return { ...actual, apiFetch: vi.fn() };
});

import { apiFetch } from '@/lib/transport/api';

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
});
