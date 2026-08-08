// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

import { MockIndicator } from '@/components/mock-indicator';

import { AppShell } from '@/features/shell';

import { Devtools } from '../components/devtools';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	return (
		<>
			<AppShell>
				{/* Inside the content column and above the outlet, NOT a third
				    grid row (spec §3.1). It describes the data, and the data is
				    here; given a row of its own it would push the rail down by
				    22px on a dev build alone, and every geometry rule in the spec
				    is written against a rail that starts at the top of the
				    webview. */}
				<MockIndicator />
				<Outlet />
			</AppShell>
			{/* A sibling of the grid rather than a child of it: the shell places
			    its three cells explicitly, so a fourth child would auto-place
			    into whichever cell is still free — in dev builds only, which is
			    the last place anyone goes looking for a layout bug. */}
			<Devtools />
		</>
	);
}
