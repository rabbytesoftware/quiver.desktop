/**
 * Pairs consecutive groupable entries two at a time; a non-groupable entry
 * (the README) or a leftover from an odd count stands alone. Order-preserving
 * and width-agnostic -- the caller decides whether to actually render grouped.
 */
export function groupTabs<T extends { groupable: boolean }>(entries: T[]): T[][] {
	const groups: T[][] = [];
	let i = 0;
	while (i < entries.length) {
		const a = entries[i];
		const b = entries[i + 1];
		if (a.groupable && b?.groupable) {
			groups.push([a, b]);
			i += 2;
		} else {
			groups.push([a]);
			i += 1;
		}
	}
	return groups;
}
