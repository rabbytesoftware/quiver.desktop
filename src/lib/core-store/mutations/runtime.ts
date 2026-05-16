import { useMutation } from '@tanstack/react-query';

import { invoke } from '@tauri-apps/api/core';

export function useInstall() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			invoke('install', { namespace, variables }),
	});
}

export function useUninstall() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			invoke('uninstall', { namespace, variables }),
	});
}

export function useExecute() {
	return useMutation({
		mutationFn: ({
			namespace,
			method,
			variables = {},
		}: {
			namespace: string;
			method: string;
			variables?: Record<string, string>;
		}) => invoke('execute', { namespace, method, variables }),
	});
}

export function useStop() {
	return useMutation({
		mutationFn: ({ namespace }: { namespace: string }) => invoke('stop', { namespace }),
	});
}
