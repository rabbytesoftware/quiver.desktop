import { apiFetch, isNotFoundError } from '@/lib/transport/api';

export interface NamespaceTarget {
	kind: 'arrow' | 'collection';
	namespace: string;
}

/**
 * `domain/user/repo[/auid]`, optionally `@version` on the last segment --
 * quiver.core's `Namespace.Validate()` shape. Cheap enough to gate the two
 * network round trips below on: an ordinary one- or two-word query never
 * reaches them.
 */
const NAMESPACE_SHAPE = /^\S+\/\S+\/\S+(\/\S+)?$/;

async function exists(path: string): Promise<boolean> {
	try {
		await apiFetch(path);
		return true;
	} catch (err) {
		if (isNotFoundError(err)) return false;
		throw err;
	}
}

/**
 * quiver.core has no combined resolve/identify endpoint -- an arrow
 * namespace and a collection namespace are syntactically identical, so the
 * only way to tell them apart (or tell that neither exists) is to ask both
 * `GET /v0/{arrow,collection}/:namespace` and see which one, if either,
 * answers. Checked in parallel; a collection wins the (rare) case where a
 * namespace happens to name both, since this app is collection-first.
 */
export async function resolveNamespaceTarget(input: string): Promise<NamespaceTarget | null> {
	const namespace = input.trim();
	if (!NAMESPACE_SHAPE.test(namespace)) return null;

	// A collection is never versioned, so a namespace carrying an `@ref` can
	// only ever name an arrow -- quiver.core answers /v0/collection for one
	// with a 500 ("internal error"), not a clean 404, since there is no such
	// thing as "not found" for a shape collections can never have. Querying
	// it anyway would let that 500 propagate (see the test for a genuine
	// server error) and fail every versioned lookup outright.
	const hasRef = namespace.includes('@');

	const [isCollection, isArrow] = await Promise.all([
		hasRef ? Promise.resolve(false) : exists(`/v0/collection/${encodeURIComponent(namespace)}`),
		exists(`/v0/arrow/${encodeURIComponent(namespace)}`),
	]);

	if (isCollection) return { kind: 'collection', namespace };
	if (isArrow) return { kind: 'arrow', namespace };
	return null;
}
