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
			<AppShell footer={<MockIndicator />}>
				<Outlet />
			</AppShell>
			<Devtools />
		</>
	);
}
