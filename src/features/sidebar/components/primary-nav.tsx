import type { JSX } from 'react';

import { GearIcon, HardDrivesIcon, HouseIcon } from '@phosphor-icons/react';
import { useRouterState } from '@tanstack/react-router';

import { Tabs, TabsList } from '@/components/ui/tabs';

import { useTranslation } from '@/lib/i18n';

import { NavSegment, type NavDestination } from './nav-segment';

interface Destination {
	value: string;
	to: NavDestination;
	/** Phosphor's component type; `PrimaryNav` only passes it through. */
	icon: typeof HouseIcon;
	/** `t()` key, resolved at render so a locale change re-labels the nav. */
	key: 'nav.home' | 'nav.remote' | 'nav.settings';
}

const DESTINATIONS: readonly Destination[] = [
	{ value: 'home', to: '/', icon: HouseIcon, key: 'nav.home' },
	{ value: 'remote', to: '/remote', icon: HardDrivesIcon, key: 'nav.remote' },
	{ value: 'settings', to: '/settings', icon: GearIcon, key: 'nav.settings' },
];

/**
 * Which destination the router is on, or `null` for none.
 *
 * `null` is the case Crowbar's own tab bar cannot express and Quiver needs:
 * its tabs are a store value that is always one of the four, whereas these
 * three are DESTINATIONS competing with every arrow in the list below. Open an
 * arrow and the answer is honestly "none of them" — Base UI then has no tab to
 * measure, drops the indicator, and the control renders as a bare track. That
 * is the whole reason the value is derived rather than held.
 *
 * Home is matched EXACTLY. TanStack matches by prefix and `/` is a prefix of
 * every route in the app, so a prefix test would light Home on top of Remote, on
 * top of Settings, on top of every open arrow. The leaves want their subtrees
 * and so test for the boundary too — `/settings/general` is still Settings,
 * while a hypothetical `/settingsomething` is not.
 */
function activeDestination(pathname: string): string | null {
	if (pathname === '/') return 'home';

	const match = DESTINATIONS.find(
		(destination) =>
			destination.to !== '/' && (pathname === destination.to || pathname.startsWith(`${destination.to}/`))
	);

	return match?.value ?? null;
}

/**
 * Home / Remote / Settings, the fixed head of the rail.
 *
 * A segmented control rather than three independent links: the three are one
 * choice, and a sliding indicator says that in a way three separately-filled
 * rows do not. Crowbar's `sidebar-tab-bar.tsx` is the reference.
 *
 * Copied from that file rather than adapted: the wrapper's `px-2 py-1.5`, the
 * track's `bg-sidebar-element-idle text-foreground/70`, the tab's
 * `flex flex-1 items-center justify-center gap-1`, the 14px glyph and the
 * label thresholds are all crowbar's.
 *
 * The one departure is theming, and it is the same one the arrow rows make:
 * CossUI's indicator fills from `--background` and lifts, Quiver's fills from
 * `--foreground` and inverts (see `components/ui/tabs.tsx`). One selection
 * language across the whole rail.
 *
 * `@container` so the labels below can respond to the RAIL's width. The window's
 * width says nothing about it: the rail is dragged between 160 and 320px.
 */
export function PrimaryNav(): JSX.Element {
	const { t } = useTranslation();
	const pathname = useRouterState({ select: (state) => state.location.pathname });
	const active = activeDestination(pathname);

	return (
		<nav className="@container flex shrink-0 items-center">
			{/* No `onValueChange`: the tabs do not own the selection. Each segment is
			    a Link, the URL changes, and `active` is re-derived from it — so the
			    control cannot get out of step with where the app actually is. */}
			<Tabs value={active} className="w-full">
				<TabsList variant="default" className="w-full bg-sidebar-element-idle text-foreground/70">
					{DESTINATIONS.map((destination) => (
						<NavSegment
							key={destination.value}
							to={destination.to}
							value={destination.value}
							active={active === destination.value}
							icon={destination.icon}
							label={t(destination.key)}
						/>
					))}
				</TabsList>
			</Tabs>
		</nav>
	);
}
