// src/routes/__root.tsx
import { createRootRoute, Outlet } from '@tanstack/react-router';

import { MockIndicator } from '@/components/mock-indicator';

import { SettingsDialog } from '@/features/settings/components/settings-dialog';
import { useSettingsUI } from '@/features/settings/store';
import { useTranslation } from '@/lib/i18n';

import { Devtools } from '../components/devtools';
import { Titlebar } from '../components/titlebar';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	const { t } = useTranslation();
	const openSettings = useSettingsUI((s) => s.openSettings);

	return (
		<div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
			<Titlebar />
			<MockIndicator />
			<main className="min-h-0 flex-1 overflow-auto">
				<Outlet />
			</main>
			{/* Temporary, but bottom-LEFT on purpose: that is where the rail will
			    put it, so nobody learns the wrong muscle memory in the meantime. */}
			<button
				type="button"
				onClick={() => openSettings()}
				className="fixed bottom-3 left-3 z-30 border border-border bg-card px-2 py-1 text-xs text-muted-foreground hover:bg-accent hover:text-accent-foreground"
			>
				{t('app.settings')}
			</button>
			<SettingsDialog />
			<Devtools />
		</div>
	);
}
