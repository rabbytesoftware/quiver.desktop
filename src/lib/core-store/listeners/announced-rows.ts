import { backend } from '@/lib/transport/backend';

const KEY_PREFIX = 'quiver:self-announced:';

/**
 * The quiver.desktop identities this app's own announces registered on one
 * daemon connection -- the only rows an announce may later forget. A row the
 * user added (a pin, another channel) is never on it, and core cannot tell
 * the two apart: both are user-installed, and the `preinstalled:` probe marks
 * either ready on a machine that has the app.
 */
export interface AnnouncedRows {
	list(): string[];
	add(identity: string): void;
	drop(identity: string): void;
}

/** The record for the active connection; `null` when the connection cannot be named, so nothing is recorded or forgotten. */
export async function announcedRows(): Promise<AnnouncedRows | null> {
	let key: string;
	try {
		const { active_id } = await backend().getConnections();
		key = `${KEY_PREFIX}${active_id}`;
	} catch (err) {
		console.error(
			'core-store: could not name this connection; quiver.desktop rows are neither recorded nor forgotten',
			err
		);
		return null;
	}
	let identities = read(key);
	const write = (next: string[]) => {
		identities = next;
		try {
			localStorage.setItem(key, JSON.stringify(next));
		} catch {
			/* private mode or a full quota: the announce itself still stands */
		}
	};
	return {
		list: () => [...identities],
		add: (identity) => {
			if (!identities.includes(identity)) write([...identities, identity]);
		},
		drop: (identity) => write(identities.filter((known) => known !== identity)),
	};
}

function read(key: string): string[] {
	try {
		const parsed: unknown = JSON.parse(localStorage.getItem(key) ?? '[]');
		return Array.isArray(parsed) ? parsed.filter((item): item is string => typeof item === 'string') : [];
	} catch {
		return [];
	}
}
