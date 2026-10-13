import type { ConnectionConfig } from '@/domain/connection';

export function filterConnections(connections: ConnectionConfig[], query: string): ConnectionConfig[] {
	const q = query.trim().toLowerCase();
	if (q === '') return connections;
	return connections.filter((connection) => connection.name.toLowerCase().includes(q));
}

/** Wraps `index` into `[0, length)`, matching arrow-key navigation over a
 *  circular list. `0` for an empty list, since there is nothing to index. */
export function clampIndex(index: number, length: number): number {
	if (length === 0) return 0;
	return ((index % length) + length) % length;
}
