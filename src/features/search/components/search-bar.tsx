import type { JSX, ReactNode } from 'react';

import { useNavigate, useRouter, useRouterState } from '@tanstack/react-router';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

export interface SearchBarProps {
	/**
	 * Rendered before the input, on the field's own surface. Task 7 passes the
	 * macOS traffic-light reserve here — spec §4.2: beside the field instead,
	 * the reserved space sits on bare `--background` next to a different
	 * surface and the seam reads as a notch cut out of the bar.
	 */
	leading?: ReactNode;
	/** The same, after the input. Nothing passes it yet. */
	trailing?: ReactNode;
}

/** Typed on `navigate`, so a rename of the route file is a compile error here. */
const RESULTS = '/search' as const;

/**
 * The search field, which IS the chrome row rather than a control sitting in
 * one (spec §4.2).
 *
 * There is no store behind it and there must not be: `?q=` on `/search` is the
 * whole state (spec §1.6), so the value below is a read of the URL. A local
 * `useState` seeded from it would be a second copy, and the two would disagree
 * the first time anyone presses the back button — the field would still show a
 * query the results no longer have.
 */
export function SearchBar({ leading, trailing }: SearchBarProps): JSX.Element {
	const navigate = useNavigate();
	const router = useRouter();
	const { t } = useTranslation();

	// `useRouterState` and not `Route.useSearch()`: the field lives above the
	// `<Outlet/>`, so it belongs to no route's match and has no `Route` to ask.
	const location = useRouterState({ select: (state) => state.location });
	const showingResults = location.pathname === RESULTS;
	const query = readQuery(location.searchStr);

	function commit(next: string) {
		if (next === '') {
			// Pop the entry the first keystroke pushed, rather than navigating
			// to a fixed route. Only `/search` is on the stack above wherever
			// the user was, so this is the one exit that leaves history exactly
			// as it was found; `navigate({ to: '/' })` would bury that page
			// under two entries and send an emptied field Home from anywhere.
			//
			// Unreachable off `/search`: an empty change event only arrives from
			// a field that had text, and the text is `?q=`, which exists nowhere
			// else.
			router.history.back();
			return;
		}

		// The FIRST navigation pushes so back returns you where you were; every
		// keystroke after replaces (spec §1.7). Pushing each one buries that
		// page under one entry per character and back walks the query backwards
		// instead of leaving.
		void navigate({ to: RESULTS, search: { q: next }, replace: showingResults });
	}

	return (
		// `bg-background/85` with a 14px backdrop blur — spec §6.4. The blur is
		// not a vibrancy effect and does not need a transparent window: it
		// blurs the content column scrolling underneath the row.
		//
		// The focused state is the `--foreground` / `--background` inversion an
		// active rail row uses, in both directions — white on dark, near-black
		// on light. NOT `--primary`: that is the accent slot the palette
		// question has not settled (spec §5.3), and spending it here settles it
		// by accident. It grows no ring either; the inversion is the focus
		// indicator.
		<div className="group flex h-(--row) w-full items-center bg-background/85 backdrop-blur-[14px] focus-within:bg-foreground focus-within:text-background">
			{leading}
			<input
				type="text"
				value={query}
				onChange={(event) => commit(event.target.value)}
				aria-label={t('search.label')}
				placeholder={t('search.placeholder')}
				className={cn(
					'h-full min-w-0 flex-1 bg-transparent outline-none placeholder:text-muted-foreground group-focus-within:placeholder:text-inherit',
					// One class per side rather than `px-(--inset)` plus a
					// conditional `pl-0`: `cn` is tailwind-merge, which does not
					// recognise a `(--custom-property)` value and so leaves both
					// `padding-left` declarations standing — which one wins is
					// then down to the order Tailwind happens to emit its
					// utilities in, not to anything in this file.
					//
					// The slot supplies its own inset (spec §4.3). Keep ours and
					// the two stack into a double gap.
					leading ? 'pl-0' : 'pl-(--inset)',
					trailing ? 'pr-0' : 'pr-(--inset)'
				)}
			/>
			<kbd
				aria-hidden="true"
				className={cn(
					'shrink-0 font-sans text-muted-foreground group-focus-within:text-inherit',
					// The hint sits between the input and the trailing slot, so
					// the inset that separates it from the typed text has to
					// move to its own leading edge once the input has dropped
					// its trailing padding — otherwise the query runs into ⌘K.
					trailing ? 'pl-(--inset)' : 'pr-(--inset)'
				)}
			>
				{t('search.shortcut')}
			</kbd>
			{trailing}
		</div>
	);
}

/**
 * `searchStr` rather than the parsed `search` object: this component sits above
 * the `<Outlet/>` and so belongs to no route's match, which leaves the router
 * typing its search as the union of every route's schema — `q` is not on all of
 * them. Parsing the raw string keeps the read honest instead of casting the
 * union away.
 *
 * Read on every route, not only on `/search`. `?q=` is declared by exactly one
 * route, so everywhere else there is no `q` to find and this is empty already;
 * gating it would add a branch that can only differ on a URL nothing in the app
 * is able to produce, and which therefore no test can reach.
 */
function readQuery(searchStr: string): string {
	return new URLSearchParams(searchStr).get('q') ?? '';
}
