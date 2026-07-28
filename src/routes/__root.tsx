// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';

import { Titlebar } from '../components/titlebar';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	return (
		<div className="flex h-screen flex-col overflow-hidden">
			<Titlebar />
			<main className="min-h-0 flex-1 overflow-auto">
				<Outlet />
			</main>
			<TanStackRouterDevtools position="bottom-right" />
		</div>
	);
}
