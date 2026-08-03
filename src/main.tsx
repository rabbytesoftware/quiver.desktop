import React from 'react';

import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ReactQueryDevtools } from '@tanstack/react-query-devtools';
import { RouterProvider, createRouter } from '@tanstack/react-router';
import ReactDOM from 'react-dom/client';

import { setupConnectionListeners } from '@/lib/connection';
import { setupListeners } from '@/lib/core-store';
import { installMock } from '@/lib/mock';
import { readMockPreference } from '@/lib/mock/preference';

import { routeTree } from './routeTree.gen';

// FIRST, before anything opens a stream or reads the cache.
//
// Which backend is installed has to be settled before `setupListeners` runs:
// that function subscribes core status, seeds the arrow cache under whatever
// connection id the backend reports, and opens two sockets. Installing a
// different backend after any of that has begun would leave live sockets and
// seeded rows belonging to a backend nobody is talking to any more — which is
// why turning the mock on reloads the page rather than swapping in place.
//
// Reads localStorage directly rather than the store: this runs earlier than any
// component could subscribe, and earlier than zustand/persist's rehydration is
// guaranteed to have settled.
const mock = readMockPreference();
if (mock.enabled) installMock(mock.scenario);

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
