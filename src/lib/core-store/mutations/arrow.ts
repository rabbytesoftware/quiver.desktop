import { useMutation } from '@tanstack/react-query';

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

export function useRemoveArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}`, { method: 'DELETE' }),
	});
}

/** `PATCH /v0/arrow/:ns`: asks core to look for a newer version now. It re-resolves against fresh refs and answers once the check has run. */
export function useCheckForUpdate() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${namespaceSegment(namespace)}`, { method: 'PATCH' }),
	});
}
