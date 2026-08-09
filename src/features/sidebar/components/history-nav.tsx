import type { JSX } from 'react';

import { useCanGoBack, useRouter } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import { useTranslation } from '@/lib/i18n';

import { ArrowLeft, ArrowRight } from 'lucide-react';

/**
/**
 * crowbar's own history buttons, from `sidebar-project-header.tsx`: a ghost
 * `icon-sm`, squared a step tighter than the rail's rows, muted until hovered.
 *
 * `hover:bg-sidebar-element-hover` rather than the button's default `bg-muted`
 * — these sit on the rail's surface, not on a page, and the element tokens are
 * the pair crowbar tuned for exactly that (8% in light, 10% in dark, because
 * black over a light surface carries further than white over a dark one).
 *
 * `disabled:opacity-30`, not the button's own 64: a dead arrow at 64% still
 * reads as pressable against the rail.
 *
 * `icon-xs` with a 2px margin rather than crowbar's `icon-sm`. Its 28px box
 * around a 16px glyph gives a hover fill noticeably larger than the mark inside
 * it — fine in crowbar's header, heavy in a 34px rail row. The margin gives the
 * 4px back, so the cluster occupies the same width and only the fill tightens.
 */
const BUTTON = 'm-0.5 shrink-0 rounded-sm text-muted-foreground hover:bg-sidebar-element-hover disabled:opacity-30';

/**
 * 16px, crowbar's number, passed as Lucide's `size` prop. Safe as a literal
 * where a `var()` would not be: Lucide writes it to the svg's width/height
 * ATTRIBUTES, and an engine that does not substitute custom properties there
 * would silently fall back to 1em. This app ships in three webviews.
 */
const GLYPH_SIZE = 16;

export function HistoryNav(): JSX.Element {
	const router = useRouter();
	const canGoBack = useCanGoBack();
	const { t } = useTranslation();

	// No `gap` on the cluster: the buttons' own margins provide the 4px between
	// them, so the fill and the spacing move together if either is retuned.
	return (
		<div className="flex shrink-0 items-center">
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={t('nav.back')}
				disabled={!canGoBack}
				onClick={() => router.history.back()}
				className={BUTTON}
			>
				<ArrowLeft size={GLYPH_SIZE} />
			</Button>
			{/*
			 * Never disabled. `RouterHistory` has `canGoBack()` and no
			 * `canGoForward()`, so the only way to grey this out is to track the
			 * history index ourselves — a copy of the router's state that drifts
			 * the first time anything navigates without going through here. A
			 * click at the end of history is a no-op; that is the cheaper bug.
			 */}
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={t('nav.forward')}
				onClick={() => router.history.forward()}
				className={BUTTON}
			>
				<ArrowRight size={GLYPH_SIZE} />
			</Button>
		</div>
	);
}
