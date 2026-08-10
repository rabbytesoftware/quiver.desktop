import type { JSX } from 'react';

import { useNavigate, useRouter, useRouterState } from '@tanstack/react-router';

import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

const RESULTS = '/search' as const;

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
	const router = useRouter();
	const { t } = useTranslation();

	const location = useRouterState({ select: (state) => state.location });
	const showingResults = location.pathname === RESULTS;
	const query = readQuery(location.searchStr);

	function commit(next: string): void {
		if (next === '') {
			router.history.back();
			return;
		}

		void navigate({ to: RESULTS, search: { q: next }, replace: showingResults });
	}

	function open(): void {
		if (showingResults) return;
		void navigate({ to: RESULTS, search: { q: '' } });
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
				value={query}
				onFocus={open}
				onChange={(event) => commit(event.target.value)}
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
