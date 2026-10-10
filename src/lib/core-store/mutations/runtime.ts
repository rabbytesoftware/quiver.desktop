import { useMutation } from '@tanstack/react-query';

import { namespaceSegment } from '@/lib/namespace';
import { apiFetch, apiRequest } from '@/lib/transport/api';

interface RuntimeMethodInput {
	namespace: string;
	method: string;
	variables?: Record<string, string>;
}

function runtimeRequest({ namespace, method, variables = {} }: RuntimeMethodInput): [string, RequestInit] {
	return [
		`/v0/runtime/${namespaceSegment(namespace)}/${method}`,
		{
			method: 'POST',
			headers: { 'Content-Type': 'application/json' },
			body: JSON.stringify({ variables }),
		},
	];
}

/** The plain request behind every lifecycle mutation, for callers that cannot hold a hook (a context menu on a list row). */
export function runtimeMethod(input: RuntimeMethodInput): Promise<void> {
	return apiFetch<void>(...runtimeRequest(input));
}

/**
 * What `POST /v0/runtime/:ns/update` did: `started` (202) -- the update runs
 * and reports over the runtime stream, like any other lifecycle method -- or
 * `current` (200), an idempotent no-op: nothing newer, and no runtime event
 * will ever follow, so nothing may wait for one.
 */
export type UpdateOutcome = 'started' | 'current';

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
		mutationFn: async ({
			namespace,
			variables = {},
		}: {
			namespace: string;
			variables?: Record<string, string>;
		}): Promise<UpdateOutcome> => {
			const { status } = await apiRequest<void>(...runtimeRequest({ namespace, method: 'update', variables }));
			return status === 202 ? 'started' : 'current';
		},
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
