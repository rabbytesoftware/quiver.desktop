// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

import { MockIndicator } from '@/components/mock-indicator';

import { Devtools } from '../components/devtools';
import { Titlebar } from '../components/titlebar';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	return (
		<div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
			<Titlebar />
			<MockIndicator />
			<main className="min-h-0 flex-1 overflow-auto">
				<Outlet />
			</main>
			<Devtools />
		</div>
	);
}
