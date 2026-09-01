import type { ArrowEntry, ArrowState } from '@/domain/arrow';
import { splitNamespace } from '@/lib/namespace';

/**
 * A dependency-graph namespace enriched with whatever the reactive catalog
 * happens to know about it. Core's `/dependencies` and `/dependents`
 * endpoints return bare `namespace@ref` strings only -- no name, icon, or
 * live state -- so this is the one place that fills those in when possible.
 */
export interface DependencyRow {
	/** Full `namespace@ref`, for navigation -- the exact identifier the graph endpoint reported. */
	namespace: string;
	name: string;
	icon: string | null;
	ref: string;
	state: ArrowState;
	/** False when this namespace isn't in the reactive catalog at all -- `name`/`icon` then fall back to the bare namespace and no icon, and `state` is a filler value callers should ignore in favor of treating it as "Discovered". */
	userInstalled: boolean;
}

function enrichDependency(namespace: string, catalog: Map<string, ArrowEntry>): DependencyRow {
	const { head, tail } = splitNamespace(namespace);
	const ref = tail.slice(1);
	const exact = catalog.get(namespace);
	const anyVersion = exact ?? [...catalog.values()].find((entry) => splitNamespace(entry.namespace).head === head);

	if (anyVersion) {
		return {
			namespace,
			name: anyVersion.name,
			icon: anyVersion.icon,
			ref,
			state: anyVersion.state,
			userInstalled: true,
		};
	}
	return { namespace, name: head, icon: null, ref, state: 'absent', userInstalled: false };
}

export function buildDependencyRows(namespaces: string[], catalog: Map<string, ArrowEntry>): DependencyRow[] {
	return namespaces.map((namespace) => enrichDependency(namespace, catalog));
}
