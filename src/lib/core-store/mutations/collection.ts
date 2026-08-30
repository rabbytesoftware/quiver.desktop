import { useMutation, useQueryClient } from '@tanstack/react-query';

import type { CollectionDetail } from '@/domain/collection';
import { apiFetch } from '@/lib/transport/api';

/** Both endpoints return no body to reconcile against, so an optimistic flip is the only signal the UI gets before settling. */
function collectionQueryKey(namespace: string) {
	return ['collection', namespace] as const;
}

export function useFollowCollection() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/collection/${encodeURIComponent(namespace)}/follow`, { method: 'POST' }),
		onMutate: async ({ namespace }) => {
			const key = collectionQueryKey(namespace);
			await queryClient.cancelQueries({ queryKey: key });
			const previous = queryClient.getQueryData<CollectionDetail>(key);
			if (previous) queryClient.setQueryData<CollectionDetail>(key, { ...previous, followed: true });
			return { previous, key };
		},
		onError: (_err, _vars, context) => {
			if (context?.previous) queryClient.setQueryData(context.key, context.previous);
		},
	});
}

export function useUnfollowCollection() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/collection/${encodeURIComponent(namespace)}/follow`, { method: 'DELETE' }),
		onMutate: async ({ namespace }) => {
			const key = collectionQueryKey(namespace);
			await queryClient.cancelQueries({ queryKey: key });
			const previous = queryClient.getQueryData<CollectionDetail>(key);
			if (previous) queryClient.setQueryData<CollectionDetail>(key, { ...previous, followed: false });
			return { previous, key };
		},
		onError: (_err, _vars, context) => {
			if (context?.previous) queryClient.setQueryData(context.key, context.previous);
		},
	});
}
