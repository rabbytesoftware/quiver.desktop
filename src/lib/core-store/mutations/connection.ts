import { useMutation } from '@tanstack/react-query';

import { invoke } from '@tauri-apps/api/core';

import type { ConnectionConfig } from '@/domain/connection';

export function useCheckRemoteHealth() {
	return useMutation({
		mutationFn: ({ url }: { url: string }) => invoke('check_remote_health', { url }),
	});
}

export function useAddConnection() {
	return useMutation({
		mutationFn: ({ name, url, code }: { name: string; url: string; code: string }) =>
			invoke<ConnectionConfig>('add_connection', { name, url, code }),
	});
}

export function useRemoveConnection() {
	return useMutation({
		mutationFn: ({ id }: { id: string }) => invoke('remove_connection', { id }),
	});
}

export function useSwitchConnection() {
	return useMutation({
		mutationFn: ({ id }: { id: string }) => invoke('switch_connection', { id }),
	});
}

export function useRenameConnection() {
	return useMutation({
		mutationFn: ({ id, name }: { id: string; name: string }) => invoke('rename_connection', { id, name }),
	});
}
