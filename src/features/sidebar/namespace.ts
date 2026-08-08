/** The two halves of a versioned namespace, ready for §5.11's truncation ladder. */
export interface NamespaceParts {
	/** `github.com/rabbyte/minecraft` — the part allowed to truncate. */
	head: string;
	/** `@v1.21.4`, or `''` when there is no version — pinned, never truncated. */
	tail: string;
}

/**
 * Split a namespace so the subtitle can shed the middle of the path and keep
 * the version (spec §5.11).
 *
 * The split is at the LAST `@` because a namespace may carry one in its path —
 * `git@host.com/x`. Splitting at the first would hand `host.com/x` to the tail,
 * which renders `shrink-0`, and the subtitle would then refuse to shrink at all
 * and overflow the rail rather than truncating inside it.
 *
 * `at === -1` rather than a truthiness check: `@v1.21.4` splits at index 0, and
 * a falsy test would put the version in the head where the path belongs.
 */
export function splitNamespace(ns: string): NamespaceParts {
	const at = ns.lastIndexOf('@');
	if (at === -1) return { head: ns, tail: '' };
	return { head: ns.slice(0, at), tail: ns.slice(at) };
}
