import { useMutation, useQueryClient } from '@tanstack/react-query';

import type { CollectionDetail } from '@/domain/collection';
import { collectionQueryKey } from '@/lib/core-store/queries/collection';
import { apiFetch } from '@/lib/transport/api';

/**
 * Follow and unfollow are the same request shape either way the boolean goes:
 * optimistic flip in `onMutate`, rollback in `onError`. Both endpoints return
 * no body to reconcile against, so `onSettled` invalidates rather than trusts
 * the optimistic value forever -- it's the only way this ever finds out the
 * request landed on a `followed` quiver.core computed differently (a second
 * tab, a server-side no-op).
 */
function useSetCollectionFollowed(method: 'POST' | 'DELETE', followed: boolean) {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/collection/${encodeURIComponent(namespace)}/follow`, { method }),
		onMutate: async ({ namespace }) => {
			const key = collectionQueryKey(namespace);
			await queryClient.cancelQueries({ queryKey: key });
			const previous = queryClient.getQueryData<CollectionDetail>(key);
			if (previous) queryClient.setQueryData<CollectionDetail>(key, { ...previous, followed });
			return { previous, key };
		},
		onError: (_err, _vars, context) => {
			if (context?.previous) queryClient.setQueryData(context.key, context.previous);
		},
		onSettled: (_data, _error, { namespace }) => {
			queryClient.invalidateQueries({ queryKey: collectionQueryKey(namespace) });
		},
	});
}

export function useFollowCollection() {
	return useSetCollectionFollowed('POST', true);
}

export function useUnfollowCollection() {
	return useSetCollectionFollowed('DELETE', false);
}
