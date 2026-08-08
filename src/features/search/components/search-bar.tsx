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
		// `bg-background/85` over a 14px backdrop blur — spec §6.4. The blur is
		// not a vibrancy effect and does not need a transparent window: it blurs
		// the content column scrolling underneath the row. It only reads as
		// anything while the plate is translucent, so focus drops it — an opaque
		// surface that still declares `backdrop-filter` costs the compositor a
		// blurred copy of the region on every scrolled frame and paints none of
		// it.
		//
		// The focused state is the `--foreground` / `--background` inversion an
		// active rail row uses, in both directions — white on dark, near-black
		// on light. NOT `--primary`: that is the accent slot the palette
		// question has not settled (spec §5.3), and spending it here settles it
		// by accident. It grows no ring either; the inversion is the focus
		// indicator.
		<div
			// The chrome row is a window handle too, for the same reason the
			// rail's top bar is: `titleBarStyle: "Overlay"` leaves macOS drawing
			// no draggable surface at all, so every pixel of row 1 that is not an
			// interactive control has to supply one.
			//
			// The INPUT is not a drag region and must not become one — it would
			// take mousedown away from focusing and from selecting text. Tauri
			// dispatches on the event target, so the plate's padding, the lens
			// and the ⌘K hint drag while the field itself behaves like a field.
			data-tauri-drag-region
			className={cn(
				'group flex h-(--row) w-full cursor-text items-center gap-[9px] bg-background/85 backdrop-blur-[14px]',
				'focus-within:bg-foreground focus-within:text-background focus-within:backdrop-filter-none',
				// The 12px belongs to the PLATE, not to the input: the lens sits
				// ahead of the input, so padding worn by the input alone leaves
				// the magnifier flush against the window's edge.
				//
				// One class per side rather than `px-[12px]` plus a conditional
				// `pl-0`. `cn` is tailwind-merge v1, built against Tailwind v3 —
				// it cannot parse this project's v4 syntax, so which of two
				// conflicting padding classes wins would come down to the order
				// Tailwind happens to emit them in rather than to anything here.
				//
				// A slot supplies its own inset (spec §4.3). Keep ours and the
				// two stack into a double gap.
				leading ? 'pl-0' : 'pl-[12px]',
				trailing ? 'pr-0' : 'pr-[12px]'
			)}
		>
			{leading}
			{/* Drawn here rather than pulled from an icon set: this geometry is the
			    design's own — a 4.13 circle on a 13px canvas with a 1.68 stroke —
			    and the nearest icon in the set is a different weight at a
			    different size. `currentColor` throughout is what makes it invert
			    with the plate; a hard-coded stroke would stay white on white the
			    moment the field takes focus. */}
			<span className="grid flex-none opacity-70">
				<svg width="13" height="13" viewBox="0 0 13 13" aria-hidden="true">
					<circle cx="5.35" cy="5.35" r="4.13" fill="none" stroke="currentColor" strokeWidth="1.68" />
					<path d="M8.56 8.56l3.37 3.37" stroke="currentColor" strokeWidth="1.68" strokeLinecap="round" />
				</svg>
			</span>
			<input
				type="text"
				value={query}
				onChange={(event) => commit(event.target.value)}
				aria-label={t('search.label')}
				placeholder={t('search.placeholder')}
				// The placeholder is the field's only label, so its colour AND its
				// opacity are both stated rather than left to a UA stylesheet —
				// browsers disagree on a default `::placeholder` opacity, and one
				// that dims it drops the italic hint two shades below
				// `--muted-foreground` on that browser alone. Focus swaps it for
				// the inverted colour at 0.45, the one tone that stays legible on
				// the inverted plate.
				className="h-full min-w-0 flex-1 bg-transparent p-0 text-[12px] font-[480] tracking-[-0.1px] outline-none placeholder:italic placeholder:text-muted-foreground placeholder:opacity-100 group-focus-within:placeholder:text-inherit group-focus-within:placeholder:opacity-[0.45]"
			/>
			{/* Gone on focus, not dimmed: the hint tells you how to reach a field
			    you are already typing in, and it is the one thing on the plate
			    that would still be competing with the query for the eye. */}
			<kbd
				aria-hidden="true"
				className="flex-none font-sans text-[9.5px] tracking-[-0.1px] text-muted-foreground group-focus-within:hidden"
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
