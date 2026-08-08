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
			{/* A flex column so the strip can take the height it needs and the
			    shell can have the rest. `h-screen` lives HERE and nowhere below:
			    two elements both claiming the viewport's height inside one
			    another is a window that scrolls by exactly the strip's 22px. */}
			<div className="flex h-screen flex-col">
				{/* Above the grid, spanning the whole window — not a cell inside
				    it. In the content column it cut the shell in two: a
				    full-width black bar wedged between the chrome row and the
				    page, with the rail running past it on one side. It describes
				    the whole app's data, so it belongs to the whole window. */}
				<MockIndicator />
				<AppShell>
					<Outlet />
				</AppShell>
			</div>
			{/* A sibling of the column rather than a child of it: the shell places
			    its three cells explicitly and this one has no cell, so as a child
			    it would auto-place into whichever is still free — in dev builds
			    only, which is the last place anyone goes looking for a layout
			    bug. */}
			<Devtools />
		</>
	);
}
