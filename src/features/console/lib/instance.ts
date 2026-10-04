import { apiFetch } from '@/lib/transport/api';
import { backend } from '@/lib/transport/backend';

import { parseCommands } from './commands';
import { createConsoleController, type ConsoleController } from './controller';
import { fetchCoreVersions } from './versions';

let instance: ConsoleController | null = null;

/** The app's one console controller, created on first use. */
export function getConsoleController(): ConsoleController {
	instance ??= createConsoleController({
		backend,
		fetchVersions: fetchCoreVersions,
		fetchCommands: async () => parseCommands(await apiFetch<unknown>('/v0/console/commands')),
	});
	return instance;
}
