export interface NamespaceParts {
	head: string;
	tail: string;
}

export function splitNamespace(ns: string): NamespaceParts {
	const at = ns.lastIndexOf('@');
	if (at === -1) return { head: ns, tail: '' };
	return { head: ns.slice(0, at), tail: ns.slice(at) };
}

/** `github.com/rabbyte/minecraft` -> `rabbyte`; the host is already the source. */
export function ownerOf(namespace: string): string {
	const parts = namespace.split('/');
	return parts.length > 2 ? parts[parts.length - 2] : parts[0];
}
