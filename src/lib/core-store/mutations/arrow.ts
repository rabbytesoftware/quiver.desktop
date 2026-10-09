import { useMutation } from '@tanstack/react-query';

import type { AvailableVersion } from '@/domain/arrow';
import { namespaceSegment } from '@/lib/namespace';
import { apiFetch } from '@/lib/transport/api';

/**
 * `POST /v0/arrow/:ns`, no body. `namespace` is the identity the new row is
 * filed under: its selector (a channel, constraint, pinned ref or commit)
 * travels in the path and never changes afterwards. A refless namespace is
 * registered under the repository's default channel.
 */
export function useRegisterArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}`, { method: 'POST' }),
	});
}

export function removeArrowRequest(namespace: string): Promise<void> {
	return apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}`, { method: 'DELETE' });
}

export function useRemoveArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => removeArrowRequest(namespace),
	});
}

/** `POST /v0/arrow/:ns/open`: core launches the installed arrow's own app, detached from the daemon. */
export function openArrowRequest(namespace: string): Promise<void> {
	return apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}/open`, { method: 'POST' });
}

export function useOpenArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => openArrowRequest(namespace),
	});
}

/**
 * `PATCH /v0/arrow/:ns`, no body: re-checks the row's selector against its
 * repository and answers what is ahead, if anything. An installed row stays
 * where it is; only the update call moves it.
 */
export function useRecheckArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<{ available?: AvailableVersion }>(`/v0/arrow/${namespaceSegment(namespace)}`, {
				method: 'PATCH',
			}),
	});
}
