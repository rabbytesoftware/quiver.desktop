export interface NamespaceParts {
	head: string;
	tail: string;
}

/**
 * Splits at the FIRST `@`, the way quiver.core's `BareNamespace()`/`Ref()`
 * do: a namespace path never carries one, while a selector may carry slashes
 * or a `*` glob, so everything after the first `@` is the selector, whole.
 */
export function splitNamespace(ns: string): NamespaceParts {
	const at = ns.indexOf('@');
	if (at === -1) return { head: ns, tail: '' };
	return { head: ns.slice(0, at), tail: ns.slice(at) };
}

export function bareNamespace(ns: string): string {
	return splitNamespace(ns).head;
}

/** The selector a catalog identity follows -- a channel, constraint, pinned ref or commit -- or `''` when refless. */
export function selectorOf(ns: string): string {
	return splitNamespace(ns).tail.slice(1);
}

export function withSelector(ns: string, selector: string): string {
	const bare = bareNamespace(ns);
	return selector ? `${bare}@${selector}` : bare;
}

/** The whole namespace as ONE path segment: slashes in the path and in the selector alike are percent-encoded. */
export function namespaceSegment(ns: string): string {
	return encodeURIComponent(ns);
}

/** `github.com/rabbyte/minecraft` -> `rabbyte`; the host is already the source. */
export function ownerOf(namespace: string): string {
	const parts = namespace.split('/');
	return parts.length > 2 ? parts[parts.length - 2] : parts[0];
}
