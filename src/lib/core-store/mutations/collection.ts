import { useMutation } from '@tanstack/react-query';

import { apiFetch } from '@/lib/transport/api';

export function useFollowCollection() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/collection/${encodeURIComponent(namespace)}/follow`, { method: 'POST' }),
	});
}

export function useUnfollowCollection() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/collection/${encodeURIComponent(namespace)}/follow`, { method: 'DELETE' }),
	});
}
