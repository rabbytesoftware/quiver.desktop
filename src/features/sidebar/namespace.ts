export interface NamespaceParts {
	head: string;
	tail: string;
}

export function splitNamespace(ns: string): NamespaceParts {
	const at = ns.lastIndexOf('@');
	if (at === -1) return { head: ns, tail: '' };
	return { head: ns.slice(0, at), tail: ns.slice(at) };
}
