import { useMutation } from '@tanstack/react-query';

import { invoke } from '@tauri-apps/api/core';

export function useFollowCollection() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => invoke('follow_collection', { namespace }),
	});
}

export function useUnfollowCollection() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => invoke('unfollow_collection', { namespace }),
	});
}
