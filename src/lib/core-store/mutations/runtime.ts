import { useMutation } from '@tanstack/react-query';

import { apiFetch } from '@/lib/transport/api';

interface RuntimeMethodInput {
	namespace: string;
	method: string;
	variables?: Record<string, string>;
}

function runtimeMethod({ namespace, method, variables = {} }: RuntimeMethodInput): Promise<void> {
	return apiFetch<void>(`/v0/runtime/${encodeURIComponent(namespace)}/${method}`, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ variables }),
	});
}

export function useInstall() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			runtimeMethod({ namespace, method: 'install', variables }),
	});
}

export function useUninstall() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			runtimeMethod({ namespace, method: 'uninstall', variables }),
	});
}

export function useStop() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			runtimeMethod({ namespace, method: 'stop', variables }),
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
		}) => runtimeMethod({ namespace, method, variables }),
	});
}
