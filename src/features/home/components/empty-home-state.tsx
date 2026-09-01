import type { JSX } from 'react';

import { MagnifyingGlassIcon, PackageIcon } from '@phosphor-icons/react';
import { Link } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import { useTranslation } from '@/lib/i18n';

/**
 * The first-run state: nothing installed, nothing followed. Per
 * docs/home-page-spec.md §7.4, the CTA navigates to `/search` rather than
 * trying to focus the sidebar's search field in place -- no new cross-component
 * plumbing, and the sidebar's own search field already mirrors the query into
 * the URL, so landing on `/search` is a real, usable starting point.
 */
export function EmptyHomeState(): JSX.Element {
	const { t } = useTranslation();

	return (
		<div className="flex min-h-[420px] flex-col items-center justify-center gap-4 px-10 text-center">
			<div className="flex size-11 items-center justify-center rounded-xl border border-border bg-muted text-muted-foreground">
				<PackageIcon aria-hidden="true" size={20} weight="regular" />
			</div>
			<div className="flex flex-col gap-1.5">
				<h1 className="text-[15px] font-semibold tracking-[-0.2px]">{t('home.empty.title')}</h1>
				<p className="max-w-[360px] text-[12.5px]/[18px] text-muted-foreground">
					{t('home.empty.description')}
				</p>
			</div>
			<Button render={<Link search={{ q: '' }} to="/search" />} size="sm">
				<MagnifyingGlassIcon aria-hidden="true" size={14} weight="bold" />
				{t('home.empty.cta')}
			</Button>
		</div>
	);
}
