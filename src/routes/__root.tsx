// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';

import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	return (
		<div className="flex h-full">
			<main className="flex-1 overflow-hidden">
				<Outlet />
			</main>
			<TanStackRouterDevtools position="bottom-right" />
		</div>
	);
}
