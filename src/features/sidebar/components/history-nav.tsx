import type { JSX } from 'react';

import { CaretLeftIcon, CaretRightIcon } from '@phosphor-icons/react';
import { useCanGoBack, useRouter } from '@tanstack/react-router';

import { useTranslation } from '@/lib/i18n';

/**
 * Shared by both buttons, and the reason `not-disabled:` is a variant rather
 * than a later `disabled:hover:bg-transparent` override: an override still lets
 * the fill paint for the frame before it wins, so the disabled back button
 * flickers as the cursor crosses it on its way to forward. Excluding never
 * paints at all. The same rule covers arrow rows and nav segments.
 *
 * Deliberately no `disabled:pointer-events-none`, which would settle the same
 * question by suppressing the event and leave nothing for `not-disabled:` to
 * exclude — the rule the rest of the rail is written against would then be
 * carrying no weight anywhere, and nothing would notice when it broke.
 *
 * `opacity-30`, not the 50 `components/ui/button.tsx` uses: these sit on the
 * rail's own surface rather than on a filled button, and at 50 a dead chevron
 * still reads as pressable against it.
 */
const BUTTON =
	'inline-flex size-(--row) shrink-0 items-center justify-center not-disabled:hover:bg-sidebar-accent disabled:opacity-30';

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
			<button
				type="button"
				aria-label={t('nav.back')}
				disabled={!canGoBack}
				onClick={() => router.history.back()}
				className={BUTTON}
			>
				<CaretLeftIcon className={GLYPH} weight="bold" />
			</button>
			{/*
			 * Never disabled. `RouterHistory` has `canGoBack()` and no
			 * `canGoForward()`, so the only way to grey this out is to track the
			 * history index ourselves — a copy of the router's state that drifts
			 * the first time anything navigates without going through here. A
			 * click at the end of history is a no-op; that is the cheaper bug.
			 */}
			<button
				type="button"
				aria-label={t('nav.forward')}
				onClick={() => router.history.forward()}
				className={BUTTON}
			>
				<CaretRightIcon className={GLYPH} weight="bold" />
			</button>
		</div>
	);
}
