import type { JSX } from 'react';

import { useNavigate, useRouter, useRouterState } from '@tanstack/react-router';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

/** Typed on `navigate`, so a rename of the route file is a compile error here. */
const RESULTS = '/search' as const;

/**
 * The field's surface, written to sit beside the rail's other controls rather
 * than in the window chrome.
 *
 * At rest it is the same track the changer sits on — `--sidebar-element-idle`,
 * a share of `--foreground`, so it composites the same over either theme. On
 * the results route it INVERTS, which is how everything else in the rail says
 * "this is where you are": a selected arrow row, and the changer's raised
 * segment. Search is a destination (spec §1.6), so it marks itself the same way.
 *
 * `--selected-edge` for the same reason the rows carry it — a 1px top edge that
 * flips with the theme, lit on the light theme's dark fill and its mirror on the
 * dark theme's light one.
 */
const FIELD = [
	// `h-9` (36) so the field is exactly as tall as the changer's track below
	// it — 32px of segment inside 2px of padding. A 32px field beside a 36px
	// track reads as a mistake at a glance.
	'group flex h-9 w-full cursor-text items-center gap-2 rounded-lg px-2',
	'bg-sidebar-element-idle text-foreground/70',
	'transition-[color,background-color,box-shadow] duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]',
	'focus-within:ring-2 focus-within:ring-ring',
	'data-[active]:bg-foreground data-[active]:text-background',
	'data-[active]:shadow-xs data-[active]:shadow-black/10',
	'data-[active]:inset-shadow-[0_1px_var(--selected-edge)]',
].join(' ');

/**
 * The rail's search field.
 *
 * It sits above the changer rather than in the window's chrome row, so every
 * way into the app — search, the three destinations, the library — is in one
 * column and reads as one list of places to go.
 *
 * There is no store behind it and there must not be: `?q=` on `/search` is the
 * whole state (spec §1.6), so the value below is a read of the URL. A local
 * `useState` seeded from it would be a second copy, and the two would disagree
 * the first time anyone presses the back button — the field would still show a
 * query the results no longer have.
 *
 * WHAT WAS REMOVED, and why. This used to carry `data-tauri-drag-region` and a
 * threshold gesture that told a press-to-focus apart from a press-to-drag,
 * because as the chrome row it was most of the window's draggable top edge.
 * Inside the rail it is neither at that edge nor the only handle — `RailTopBar`
 * is — and a drag region fires on `mousedown` with no threshold, which would
 * take the press away from the navigation below. The rail's top strip still
 * moves the window.
 */
export function SearchBar(): JSX.Element {
	const navigate = useNavigate();
	const router = useRouter();
	const { t } = useTranslation();

	// `useRouterState` and not `Route.useSearch()`: the field lives above the
	// `<Outlet/>`, so it belongs to no route's match and has no `Route` to ask.
	const location = useRouterState({ select: (state) => state.location });
	const showingResults = location.pathname === RESULTS;
	const query = readQuery(location.searchStr);

	function commit(next: string): void {
		if (next === '') {
			// Pop the entry that opening search pushed, rather than navigating to
			// a fixed route. Only `/search` is on the stack above wherever the
			// user was, so this is the one exit that leaves history exactly as it
			// was found; `navigate({ to: '/' })` would bury that page under two
			// entries and send an emptied field Home from anywhere.
			router.history.back();
			return;
		}

		// The FIRST navigation pushes so back returns you where you were; every
		// keystroke after replaces (spec §1.7). Pushing each one buries that page
		// under one entry per character and back walks the query backwards
		// instead of leaving.
		void navigate({ to: RESULTS, search: { q: next }, replace: showingResults });
	}

	/**
	 * Opening the field IS navigating to search.
	 *
	 * On `focus` rather than `click`, so the keyboard reaches it too — tabbing in
	 * and the shortcut this field advertises both land here, without a second
	 * handler that could drift from this one.
	 *
	 * Guarded on `showingResults`, or every re-focus pushes another entry and the
	 * back button walks a stack of identical URLs.
	 */
	function open(): void {
		if (showingResults) return;
		void navigate({ to: RESULTS, search: { q: '' } });
	}

	return (
		<div className={FIELD} {...(showingResults ? { 'data-active': '' } : {})}>
			{/* Drawn here rather than pulled from an icon set: this geometry is the
			    design's own — a 4.13 circle on a 13px canvas with a 1.68 stroke —
			    and the nearest icon in the set is a different weight at a different
			    size. `currentColor` throughout is what makes it invert with the
			    field; a hard-coded stroke would stay dark on dark the moment the
			    route becomes the active one. */}
			<span className="grid flex-none" aria-hidden="true">
				<svg width="13" height="13" viewBox="0 0 13 13">
					<circle cx="5.35" cy="5.35" r="4.13" fill="none" stroke="currentColor" strokeWidth="1.68" />
					<path d="M8.56 8.56l3.37 3.37" stroke="currentColor" strokeWidth="1.68" strokeLinecap="round" />
				</svg>
			</span>
			<input
				type="text"
				value={query}
				onFocus={open}
				onChange={(event) => commit(event.target.value)}
				aria-label={t('search.label')}
				placeholder={t('search.placeholder')}
				// The placeholder is the field's only label, so its colour AND its
				// opacity are both stated rather than left to a UA stylesheet —
				// browsers disagree on a default `::placeholder` opacity, and one
				// that dims it drops the hint a further shade on that browser alone.
				// `text-inherit` carries it through the inversion.
				className={cn(
					'h-full min-w-0 flex-1 bg-transparent p-0 text-[13px]/[16px] outline-none',
					'placeholder:text-inherit placeholder:opacity-100'
				)}
			/>
			{/* Gone once search IS the route: the hint tells you how to reach a
			    field you are already in, and it is the one thing left competing
			    with the query for the eye. */}
			<kbd
				aria-hidden="true"
				className="flex-none font-sans text-[10px] tracking-[-0.1px] opacity-60 group-data-[active]:hidden"
			>
				{t('search.shortcut')}
			</kbd>
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
