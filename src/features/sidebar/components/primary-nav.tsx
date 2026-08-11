import type { JSX } from 'react';

import { GearIcon, HardDrivesIcon, HouseIcon } from '@phosphor-icons/react';
import { useRouterState } from '@tanstack/react-router';

import { Tabs, TabsList } from '@/components/ui/tabs';

import { useTranslation } from '@/lib/i18n';

import { NavSegment, type NavDestination } from './nav-segment';

interface Destination {
	value: string;
	to: NavDestination;
	icon: typeof HouseIcon;
	key: 'nav.home' | 'nav.remote' | 'nav.settings';
}

const DESTINATIONS: readonly Destination[] = [
	{ value: 'home', to: '/', icon: HouseIcon, key: 'nav.home' },
	{ value: 'remote', to: '/remote', icon: HardDrivesIcon, key: 'nav.remote' },
	{ value: 'settings', to: '/settings', icon: GearIcon, key: 'nav.settings' },
];

function activeDestination(pathname: string): string | null {
	if (pathname === '/') return 'home';

	const match = DESTINATIONS.find(
		(destination) =>
			destination.to !== '/' && (pathname === destination.to || pathname.startsWith(`${destination.to}/`))
	);

	return match?.value ?? null;
}

export function PrimaryNav(): JSX.Element {
	const { t } = useTranslation();
	const pathname = useRouterState({ select: (state) => state.location.pathname });
	const active = activeDestination(pathname);

	return (
		<nav className="@container flex shrink-0 items-center">
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
