import { execFileSync } from 'node:child_process';
import fs from 'node:fs';

import { processAlive } from './core-api';

function pgrep(args: string[]): number[] {
	try {
		return execFileSync('pgrep', args, { encoding: 'utf8' })
			.split('\n')
			.filter(Boolean)
			.map(Number);
	} catch {
		return [];
	}
}

/** Every `quiver ... daemon` process. A core binary is `quiver` beside the app and again under <home>/self, hence the loose name. */
export function daemonPids(): number[] {
	return pgrep(['-f', 'quiver[^ ]* daemon']);
}

/** Every running desktop app process (Tauri's own process name). */
export function appPids(): number[] {
	return pgrep(['-x', 'quiverdesktop']);
}

export async function waitUntilGone(read: () => number[], what: string, timeoutMs = 30_000): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	let seen: number[] = read();
	while (seen.length > 0 && Date.now() < deadline) {
		await new Promise((r) => setTimeout(r, 250));
		seen = read();
	}
	if (seen.length > 0) throw new Error(`${what} still running after ${timeoutMs}ms: pids ${seen.join(', ')}`);
}

/** The executable a process was started from, as the kernel reports it. */
export function executableOf(pid: number): string | null {
	try {
		return fs.readlinkSync(`/proc/${pid}/exe`);
	} catch {
		return null;
	}
}

export { processAlive };

/** Asks the window manager to close the app's window, as the close button does. */
export function closeAppWindow(): void {
	execFileSync('wmctrl', ['-c', 'Quiver']);
}
