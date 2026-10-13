import { createRootRoute, Outlet } from '@tanstack/react-router';

import { MockIndicator } from '@/components/mock-indicator';

import { CommandPalette, ToastStack } from '@/features/remote';
import { AppShell } from '@/features/shell';

import { Devtools } from '../components/devtools';
import '../index.css';

export const Route = createRootRoute({
	component: RootLayout,
});

function RootLayout() {
	return (
		<>
			<AppShell footer={<MockIndicator />}>
				<Outlet />
			</AppShell>
			<CommandPalette />
			<ToastStack />
			<Devtools />
		</>
	);
}
