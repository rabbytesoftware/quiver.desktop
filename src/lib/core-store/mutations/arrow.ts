import { useMutation } from '@tanstack/react-query';

import { invoke } from '@tauri-apps/api/core';

export function useRegisterArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => invoke('register_arrow', { namespace }),
	});
}

export function useRemoveArrow() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => invoke('remove_arrow', { namespace }),
	});
}
