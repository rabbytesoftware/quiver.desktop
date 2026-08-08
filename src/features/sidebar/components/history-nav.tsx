import type { JSX } from 'react';

import { CaretLeftIcon, CaretRightIcon } from '@phosphor-icons/react';
import { useCanGoBack, useRouter } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import { useTranslation } from '@/lib/i18n';

/**
/**
 * Sizing and the disabled tone only; `variant="ghost"` supplies the rest, so
 * these read as the same control as every other ghost button in the app.
 *
 * `opacity-30`, not the 64 the button's own `disabled:` sets: these sit on the
 * rail's surface rather than on a filled button, and at 64 a dead chevron still
 * reads as pressable against it.
 *
 * `rounded-md`, a step tighter than the rail's rows — a 34px square at
 * `rounded-lg` is nearly a pill.
 */
const BUTTON = 'size-(--row) rounded-md disabled:opacity-30';

/**
 * `--icon-chrome`, one step below the list's `--icon`; see spec §3.2.
 *
 * Sized by class rather than by Phosphor's `size` prop: the prop takes a number
 * or a CSS length string, and neither can carry `var(--icon-chrome)` — passing
 * one hardcodes 17 in a second place and the glyphs stop following the token
 * the next time the scale moves. The rendered `<svg>` has no intrinsic size to
 * fight, so the class wins outright.
 */
const GLYPH = 'size-(--icon-chrome)';

/**
 * Back and forward for the rail's top bar.
 *
 * `useCanGoBack()` rather than `router.history.canGoBack()`: the latter is a
 * plain read of a mutable closure, and this component lives in the rail — above
 * the `<Outlet/>`, so nothing re-renders it when the route changes. The button
 * would freeze in whatever state it had at mount, disabled for the whole
 * session. The hook is the same predicate over the router's own location store,
 * so it is a subscription, not a second copy of the history's state.
 */
export function HistoryNav(): JSX.Element {
	const router = useRouter();
	const canGoBack = useCanGoBack();
	const { t } = useTranslation();

	return (
		<div className="flex">
			<Button
				variant="ghost"
				size="icon"
				aria-label={t('nav.back')}
				disabled={!canGoBack}
				onClick={() => router.history.back()}
				className={BUTTON}
			>
				<CaretLeftIcon className={GLYPH} weight="bold" />
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
				size="icon"
				aria-label={t('nav.forward')}
				onClick={() => router.history.forward()}
				className={BUTTON}
			>
				<CaretRightIcon className={GLYPH} weight="bold" />
			</Button>
		</div>
	);
}
