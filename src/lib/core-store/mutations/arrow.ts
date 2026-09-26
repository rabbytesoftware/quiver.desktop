import { useMutation } from '@tanstack/react-query';

import { apiFetch } from '@/lib/transport/api';

/**
 * `channel` pins the channel to install at, at registration time -- omitted
 * from the request body entirely (not sent as `null`) when the caller picked
 * none, matching this call's existing no-body behaviour for that common case.
 */
export function useRegisterArrow() {
	return useMutation({
		mutationFn: ({ namespace, channel }: { namespace: string; channel?: string }) =>
			apiFetch<void>(
				`/v0/arrow/${encodeURIComponent(namespace)}`,
				channel !== undefined
					? {
							method: 'POST',
							headers: { 'Content-Type': 'application/json' },
							body: JSON.stringify({ channel }),
						}
					: { method: 'POST' }
			),
	});
}

export function useRemoveArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, { method: 'DELETE' }),
	});
}

/**
 * `PATCH /v0/arrow/:ns` -- switches an already-installed arrow to a
 * different channel and/or pins a specific ref within it. Not to be confused
 * with `useUpdate` (runtime.ts): that runs the arrow's own manifest-defined
 * `_update` lifecycle script, while this is quiver.core's catalog-level
 * "which ref is this arrow installed at" operation. `ref` is omitted from
 * the body when not given -- `JSON.stringify` drops an `undefined` property
 * on its own, so core resolves the channel's own `latest` itself.
 */
export function useSwitchArrowChannel() {
	return useMutation({
		mutationFn: ({ namespace, channel, ref }: { namespace: string; channel: string; ref?: string }) =>
			apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, {
				method: 'PATCH',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({ channel, ref }),
			}),
	});
}
