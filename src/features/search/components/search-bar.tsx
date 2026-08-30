import type { JSX } from 'react';
import { useEffect, useRef, useState } from 'react';

import { useNavigate, useRouterState } from '@tanstack/react-router';

import { resolveNamespaceTarget } from '@/features/search/api/resolve-namespace';
import { cn } from '@/lib/cn';
import { useSearchStore } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';

const RESULTS = '/search' as const;

/** Spec 2.5: the URL stays the source of truth; the debounce sits in front of it. */
export const LOCAL_DEBOUNCE_MS = 150;

const FIELD = [
	'group flex h-9 w-full cursor-text items-center gap-2 rounded-lg px-2',
	'bg-sidebar-element-idle text-foreground/70',
	'transition-[color,background-color,box-shadow] duration-200 ease-[cubic-bezier(0.4,0,0.2,1)]',
	'focus-within:ring-2 focus-within:ring-ring',
	'data-[active]:bg-foreground data-[active]:text-background',
	'data-[active]:shadow-xs data-[active]:shadow-black/10',
	'data-[active]:inset-shadow-[0_1px_var(--selected-edge)]',
].join(' ');

export function SearchBar(): JSX.Element {
	const navigate = useNavigate();
	const { t } = useTranslation();

	const location = useRouterState({ select: (state) => state.location });
	const showingResults = location.pathname === RESULTS;
	const query = readQuery(location.searchStr);

	const [draft, setDraft] = useState(query);
	const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

	function clearPending(): void {
		if (timer.current !== null) clearTimeout(timer.current);
		timer.current = null;
	}

	// The URL is authoritative while it is naming a query: a back button or a deep
	// link overwrites the draft and discards whatever commit was in flight, so a
	// stale timer can't undo the navigation.
	//
	// Off the results route it names nothing, and syncing there would blank the
	// field on the way out -- throwing the query away and leaving `open` below
	// nothing to reopen but an empty search. This field lives in the sidebar and
	// outlives the route, so the query it is holding outlives it too.
	useEffect(() => {
		clearPending();
		if (showingResults) setDraft(query);
	}, [query, showingResults]);

	useEffect(() => () => clearPending(), []);

	function commit(next: string): void {
		// Emptying the field asks for an empty search, not for a different page.
		// This popped history instead, which landed wherever the stack happened
		// to point: home, an older search -- refilling the field with the query
		// just deleted -- or the arrow page you had come back from. Replacing
		// keeps the cursor where it is, on a results route with nothing in it.
		if (next === '') {
			if (showingResults) void navigate({ to: RESULTS, search: { q: '' }, replace: true });
			return;
		}

		void navigate({ to: RESULTS, search: { q: next }, replace: showingResults });
	}

	function change(next: string): void {
		setDraft(next);
		clearPending();
		timer.current = setTimeout(() => {
			timer.current = null;
			commit(next);
		}, LOCAL_DEBOUNCE_MS);
	}

	async function submit(): Promise<void> {
		clearPending();
		const trimmed = draft.trim();

		// Enter is also "take me there": if what's typed names a real arrow or
		// collection, that beats a search for it every time. Only on Enter --
		// resolving on every keystroke would fire two lookups per character.
		if (trimmed !== '') {
			const target = await resolveNamespaceTarget(trimmed).catch(() => null);
			if (target !== null) {
				void navigate({
					to: target.kind === 'collection' ? '/collection/$' : '/arrow/$',
					params: { _splat: target.namespace },
				});
				return;
			}
		}

		// Spec 2.2: Enter fires the pass now, not after 600ms of stillness. The
		// mounted controller lives behind the /search route, a sibling of this
		// field, so the request travels through the store.
		if (trimmed !== '') useSearchStore.getState().requestSubmit(draft);
		commit(draft);
	}

	// Focusing reopens whatever the field is still holding, so the field and the
	// results always name the same query. Clearing the field is what asks for an
	// empty search, and the only thing that does.
	function open(): void {
		if (showingResults) return;
		// Reopening restores a screen rather than asking for a search: Lane A
		// rebuilds it from the vault the last pass filled, and the git hosts are
		// left alone until the query actually changes or Enter asks for them.
		useSearchStore.getState().requestRestore(draft);
		void navigate({ to: RESULTS, search: { q: draft } });
	}

	return (
		<div className={FIELD} {...(showingResults ? { 'data-active': '' } : {})}>
			<span className="grid flex-none" aria-hidden="true">
				<svg width="13" height="13" viewBox="0 0 13 13">
					<circle cx="5.35" cy="5.35" r="4.13" fill="none" stroke="currentColor" strokeWidth="1.68" />
					<path d="M8.56 8.56l3.37 3.37" stroke="currentColor" strokeWidth="1.68" strokeLinecap="round" />
				</svg>
			</span>
			<input
				type="text"
				value={draft}
				onFocus={open}
				onChange={(event) => change(event.target.value)}
				onKeyDown={(event) => {
					if (event.key === 'Enter') void submit();
				}}
				aria-label={t('search.label')}
				placeholder={t('search.placeholder')}
				className={cn(
					'h-full min-w-0 flex-1 bg-transparent p-0 text-[13px]/[16px] outline-none',
					'placeholder:text-inherit placeholder:opacity-100'
				)}
			/>
		</div>
	);
}

function readQuery(searchStr: string): string {
	return new URLSearchParams(searchStr).get('q') ?? '';
}
