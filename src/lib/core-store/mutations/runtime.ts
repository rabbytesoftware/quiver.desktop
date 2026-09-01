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

export function useUpdate() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			runtimeMethod({ namespace, method: 'update', variables }),
	});
}

/**
 * The one universal "go" action -- `Target.Lifecycle.Execute`, always this
 * exact call, never a custom method name. Hard-gated to `ready` only by core
 * itself (`BeginExecution.Validate`); no manifest override is possible, so
 * don't add a state check here that could drift from that.
 */
export function useExecuteArrow() {
	return useMutation({
		mutationFn: ({ namespace, variables = {} }: { namespace: string; variables?: Record<string, string> }) =>
			runtimeMethod({ namespace, method: 'execute', variables }),
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
