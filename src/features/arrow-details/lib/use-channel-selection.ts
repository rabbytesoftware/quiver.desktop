import { useState } from 'react';

import type { ArrowChannel } from '@/domain/arrow';

export interface ChannelSelection {
	/** The channel currently shown in the Channel select. */
	selectedChannel: string | undefined;
	/** The version currently shown in the Version select, scoped to `selectedChannel`. */
	selectedVersion: string | undefined;
	/** The `channels` entry matching `selectedChannel`, or undefined when the arrow has no channels at all. */
	selectedChannelEntry: ArrowChannel | undefined;
	/** The selector the current pick registers as -- see `selectorFor`. Undefined while there is nothing to pick from. */
	selector: string | undefined;
	/** Picks a different channel, resetting the version to that channel's own newest. */
	selectChannel(name: string): void;
	/** Picks a different version within the selected channel. */
	selectVersion(ref: string): void;
}

/** The default version within a channel -- its own `latest` for a pointer channel, `members[0]` (already the highest-precedence entry) for an ordered one. */
function defaultVersionOf(channel: ArrowChannel | undefined): string | undefined {
	if (!channel) return undefined;
	return channel.kind === 'ordered' ? (channel.members?.[0] ?? channel.latest) : channel.latest;
}

/**
 * The selector a Channel + Version pick means. Keeping a channel's newest
 * version follows the channel (`crowbar@stable`), so the row keeps moving
 * with it; anything else is a pin of exactly that ref (`crowbar@v1.2.0`).
 */
export function selectorFor(channel: ArrowChannel | undefined, version: string | undefined): string | undefined {
	if (!channel) return undefined;
	if (version === undefined || version === defaultVersionOf(channel)) return channel.name;
	return version;
}

function seed(selector: string, channels: ArrowChannel[]): { channel?: ArrowChannel; version?: string } {
	const named = channels.find((c) => c.name === selector);
	if (named) return { channel: named, version: defaultVersionOf(named) };
	const listing = channels.find((c) => c.members?.includes(selector));
	if (listing) return { channel: listing, version: selector };
	return { channel: channels[0], version: defaultVersionOf(channels[0]) };
}

/**
 * Local Channel/Version pick, starting from `selector` (the one the page's
 * identity follows) and scoped to `channels` -- what the repository
 * publishes. Picking changes nothing on core: a catalog row's selector never
 * changes, so a pick only ever becomes the identity of a row that does not
 * exist yet (Add to Library, or the switch dialog's reinstall).
 */
export function useChannelSelection(selector: string, channels: ArrowChannel[]): ChannelSelection {
	// Re-seeded once when `channels` arrives: it comes from its own slower
	// query and is `[]` on the first render for the same identity.
	const seedKey = `${selector}#${channels.length > 0}`;
	const [seededFor, setSeededFor] = useState<string | null>(null);
	const [selectedChannel, setSelectedChannel] = useState<string | undefined>(undefined);
	const [selectedVersion, setSelectedVersion] = useState<string | undefined>(undefined);
	if (seededFor !== seedKey) {
		setSeededFor(seedKey);
		const initial = seed(selector, channels);
		setSelectedChannel(initial.channel?.name);
		setSelectedVersion(initial.version);
	}
	const selectedChannelEntry = channels.find((c) => c.name === selectedChannel);

	return {
		selectedChannel,
		selectedVersion,
		selectedChannelEntry,
		selector: selectorFor(selectedChannelEntry, selectedVersion),
		selectChannel(name: string): void {
			setSelectedChannel(name);
			setSelectedVersion(defaultVersionOf(channels.find((c) => c.name === name)));
		},
		selectVersion(ref: string): void {
			setSelectedVersion(ref);
		},
	};
}
