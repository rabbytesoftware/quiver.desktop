import { useMutation } from '@tanstack/react-query';

import { apiFetch } from '@/lib/transport/api';

export function useRegisterArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, { method: 'POST' }),
	});
}

export function useRemoveArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) =>
			apiFetch<void>(`/v0/arrow/${encodeURIComponent(namespace)}`, { method: 'DELETE' }),
	});
}
