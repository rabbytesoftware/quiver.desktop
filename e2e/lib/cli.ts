import { spawnSync } from 'node:child_process';

import { quiverHome, selfInstalledCore } from './paths';

export interface CliResult {
	status: number | null;
	stdout: string;
	stderr: string;
}

/**
 * Runs the `quiver` CLI a user would type, against the daemon the app under
 * test is running: the self-installed binary, with the same HOME the app was
 * given so both reach one socket. JSON output is selected by the CLI itself
 * whenever stdout is not a terminal.
 */
export function quiverCli(home: string, args: string[]): CliResult {
	const result = spawnSync(selfInstalledCore(home), args, {
		encoding: 'utf8',
		env: { ...process.env, HOME: home, QUIVER_HOME: quiverHome(home) },
		timeout: 120_000,
	});
	return { status: result.status, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
}
