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
			{/* The band goes in the content column's footer: bottom of the page,
			    and stopping at the rail rather than running across it. Spanning
			    the window put it against the traffic lights; between the chrome
			    row and the page it cut the shell in half. Below the content it is
			    out of the way of both. */}
			<AppShell footer={<MockIndicator />}>
				<Outlet />
			</AppShell>
			{/* A sibling of the column rather than a child of it: the shell places
			    its three cells explicitly and this one has no cell, so as a child
			    it would auto-place into whichever is still free — in dev builds
			    only, which is the last place anyone goes looking for a layout
			    bug. */}
			<Devtools />
		</>
	);
}
