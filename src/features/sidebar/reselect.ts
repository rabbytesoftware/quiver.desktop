import type { MouseEvent } from 'react';

/**
 * Clicking the rail row you are already on (spec §5.9).
 *
 * Without this, clicking Home twice leaves a back button that appears broken:
 * the entry behind you is the page you are already looking at, so pressing Back
 * re-renders the same screen and nothing moves. One click per row is enough to
 * do it, there is nothing on screen that says how many identical entries are
 * stacked up, and the user's conclusion is that Back is dead.
 *
 * The test is `data-status`, which the ROUTER writes on the active link — the
 * same attribute the active fill and the namespace subtitle already key off. A
 * `useMatchRoute` call here would be a second route matcher with its own
 * spelling of `exact`, free to disagree with the `activeOptions` on the very
 * link it is attached to: a row that lights up and still pushes, or one that
 * refuses to navigate while looking inactive.
 *
 * `preventDefault` rather than `replace: true` because `replace` is a prop on
 * `<Link>` and `isActive` is only in scope inside its children function — every
 * way of hoisting it back out is the second matcher above. It also does less:
 * replacing re-runs the loaders of the route already open.
 */
export function blockReselect(event: MouseEvent<HTMLAnchorElement>): void {
	// TanStack composes this handler ahead of its own and skips the rest of the
	// chain once the event is prevented, so this cancels the NAVIGATION and not
	// merely the browser's default. Reordering that in a future version would
	// leave the push in place with nothing here failing — the tests in
	// sidebar.test.tsx are what notice.
	if (event.currentTarget.dataset.status === 'active') event.preventDefault();
}
