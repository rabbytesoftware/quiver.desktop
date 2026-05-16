import { getCurrentWindow } from '@tauri-apps/api/window';
import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/react-router-devtools';
import { useEffect } from 'react';

import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	useEffect(() => {
		const appWindow = getCurrentWindow();
		const onMouseDown = (e: MouseEvent) => {
			const target = e.target as HTMLElement;
			if (target.closest('[data-tauri-drag-region]')) {
				appWindow.startDragging();
			}
		};
		document.addEventListener('mousedown', onMouseDown);
		return () => document.removeEventListener('mousedown', onMouseDown);
	}, []);

	return (
		<>
			<Outlet />
			<TanStackRouterDevtools position="bottom-right" />
		</>
	);
}
