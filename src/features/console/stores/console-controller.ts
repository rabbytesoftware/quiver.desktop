
import { parseCommands } from '@/features/console/lib/commands';
import { createConsoleController, type ConsoleController } from '@/features/console/lib/controller';
import { fetchCoreVersions } from '@/features/console/lib/versions';
import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

import { useConsoleStore } from './console-store';

let instance: ConsoleController | null = null;

/**
 * The app's one console controller, created on first use: the pure controller in
 * `lib/` wired to the console store and to the real backend. It lives beside the
 * store because wiring a store to the thing that drives it is the store's business,
 * and `lib/` is kept free of both.
 */
export function getConsoleController(): ConsoleController {
	instance ??= createConsoleController({
		store: useConsoleStore,
		backend,
		fetchVersions: fetchCoreVersions,
		fetchCommands: async () => parseCommands(await apiFetch<unknown>('/v0/console/commands')),
	});
	return instance;
}
