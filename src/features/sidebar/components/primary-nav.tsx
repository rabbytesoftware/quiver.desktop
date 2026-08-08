import type { JSX } from 'react';

import { GearIcon, HardDrivesIcon, HouseIcon } from '@phosphor-icons/react';

import { useTranslation } from '@/lib/i18n';

import { NavSegment } from './nav-segment';

/**
 * Home / Remote / Settings, the fixed head of the rail.
 *
 * There is no selection state here and no `activeIndex` to keep in step with the
 * arrow list below: each segment is a `<Link>` and the router marks the one that
 * matches. "Exactly one thing in the rail is active" stops being an invariant
 * this component enforces and becomes one it cannot violate.
 */
export function PrimaryNav(): JSX.Element {
	const { t } = useTranslation();

	return (
		<nav className="flex">
			{/*
			 * `exact` is load-bearing, and it is the one line here worth reading
			 * twice. TanStack matches by prefix, so `/` is a prefix of every route
			 * in the app: without it Home is lit on top of Remote, on top of
			 * Settings, on top of every open arrow — and the rail's whole selection
			 * rule is broken on the first click. The leaves want prefix matching
			 * and so pass nothing.
			 */}
			<NavSegment to="/" exact icon={HouseIcon} label={t('nav.home')} />
			<NavSegment to="/remote" icon={HardDrivesIcon} label={t('nav.remote')} />
			<NavSegment to="/settings" icon={GearIcon} label={t('nav.settings')} />
		</nav>
	);
}
