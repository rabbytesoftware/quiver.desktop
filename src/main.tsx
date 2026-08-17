import React from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider, createRouter } from '@tanstack/react-router';
import ReactDOM from 'react-dom/client';

import { installThemeSync } from '@/features/shell';
import { setupConnectionListeners } from '@/lib/connection';
import { setupListeners } from '@/lib/core-store';
import { installLocaleSync } from '@/lib/i18n';
import { installMock } from '@/lib/mock';
import { readMockPreference } from '@/lib/mock/preference';

import { routeTree } from './routeTree.gen';

const mock = readMockPreference();
if (mock.enabled) installMock(mock.scenario);

installLocaleSync();
installThemeSync();

setupListeners();
setupConnectionListeners();

const queryClient = new QueryClient({
	defaultOptions: {
		queries: {
			staleTime: 30_000,
			retry: 1,
		},
	},
});

const router = createRouter({ routeTree });

declare module '@tanstack/react-router' {
	interface Register {
		router: typeof router;
	}
}

ReactDOM.createRoot(document.getElementById('root')!).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
			<ReactQueryDevtools initialIsOpen={false} />
		</QueryClientProvider>
	</React.StrictMode>
);
