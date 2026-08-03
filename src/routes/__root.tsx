// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

import { MockIndicator } from '@/components/mock-indicator';

import { SettingsDialog } from '@/features/settings/components/settings-dialog';
import { useSettingsUI } from '@/features/settings/store';

import { Devtools } from '../components/devtools';
import { Titlebar } from '../components/titlebar';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	const openSettings = useSettingsUI((s) => s.openSettings);

	return (
		<div className="flex h-screen flex-col overflow-hidden bg-ground text-ink">
			<Titlebar />
			<MockIndicator />
			<main className="min-h-0 flex-1 overflow-auto">
				<Outlet />
			</main>
			{/* Temporary: the settings dialog has no home yet, because the rail that
			    is supposed to hold its entry point has not been built. A plain
			    corner button beats an unreachable dialog, and it comes out the
			    moment the rail lands. */}
			<button
				type="button"
				onClick={() => openSettings()}
				className="fixed bottom-3 right-3 z-30 border border-line bg-plate px-2 py-1 text-[12px] text-ink-2 hover:bg-hover hover:text-ink"
			>
				Settings
			</button>
			<SettingsDialog />
			<Devtools />
		</div>
	);
}
