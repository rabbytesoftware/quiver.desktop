import { useState } from 'react';

import { useQueryClient } from '@tanstack/react-query';

import type { ArrowChannel, ArrowDetail } from '@/domain/arrow';
import { useSwitchArrowChannel } from '@/lib/core-store';
import { arrowDetailQueryKeyPrefix } from '@/lib/core-store/queries/arrow';

export interface ChannelSelection {
	/** The channel currently shown in the Channel select -- local state, not necessarily what core has on record yet. */
	selectedChannel: string | undefined;
	/** The version currently shown in the Version select, scoped to `selectedChannel`. */
	selectedVersion: string | undefined;
	/** `detail.channels` entry matching `selectedChannel`, or undefined when the arrow has no channels at all. */
	selectedChannelEntry: ArrowChannel | undefined;
	/** True while a channel/version switch is in flight against core -- callers disable the selects on this. */
	isPending: boolean;
	/** Picks a different channel, resetting the version to that channel's own default. */
	selectChannel(name: string): void;
	/** Picks a different version within the given (already-selected) channel. */
	selectVersion(channel: string, ref: string): void;
}

/** The default version within a channel -- its own `latest` for a pointer channel, `members[0]` (already the highest-precedence entry) for an ordered one. */
function defaultVersionOf(channel: ArrowChannel | undefined): string | undefined {
	if (!channel) return undefined;
	return channel.kind === 'ordered' ? channel.members?.[0] : channel.latest;
}

/**
 * Local Channel/Version selection state for the Hero, plus the side effect of
 * switching an already-installed arrow's channel against core. Answers "what
 * channel is THIS install tracking, what's available inside it" -- scoped
 * entirely to `detail.channels`, never the reactive store's other installed
 * copies of the arrow (that question belonged to the old version switcher
 * this replaced).
 *
 * Not-yet-installed arrows get purely local state here: the caller is
 * responsible for threading `selectedChannel` into the register call itself,
 * since core's `POST /v0/arrow/:ns` only ever pins a channel, never a ref.
 */
export function useChannelSelection(detail: ArrowDetail): ChannelSelection {
	const queryClient = useQueryClient();
	const switchChannel = useSwitchArrowChannel();

	// Seeded from `detail.channel` (or the first published channel, when
	// installed at no tracked channel at all) once per arrow -- the same
	// render-time "adjusting state when a prop changes" pattern
	// arrow-details-screen.tsx already uses for its own per-arrow local state,
	// so the very first paint already shows a real selection instead of
	// nothing. Not re-derived from `detail` on every render: once the user (or
	// a successful switch) has picked something, that choice stays authoritative
	// until a different arrow is shown.
	const [seededFor, setSeededFor] = useState<string | null>(null);
	const [selectedChannel, setSelectedChannel] = useState<string | undefined>(undefined);
	const [selectedVersion, setSelectedVersion] = useState<string | undefined>(undefined);
	if (seededFor !== detail.namespace) {
		setSeededFor(detail.namespace);
		const initialChannel = detail.channel || detail.channels[0]?.name;
		setSelectedChannel(initialChannel);
		setSelectedVersion(defaultVersionOf(detail.channels.find((c) => c.name === initialChannel)));
	}
	const selectedChannelEntry = detail.channels.find((c) => c.name === selectedChannel);

	function switchOnCore(channel: string, ref: string | undefined): void {
		if (!detail.user_installed) return;
		switchChannel
			.mutateAsync({ namespace: detail.namespace, channel, ref })
			.then(() => queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix }))
			.catch(() => {});
	}

	function selectChannel(name: string): void {
		const entry = detail.channels.find((c) => c.name === name);
		const version = defaultVersionOf(entry);
		setSelectedChannel(name);
		setSelectedVersion(version);
		switchOnCore(name, version);
	}

	// Takes the channel explicitly (the Version select only ever renders
	// alongside a resolved `selectedChannelEntry`, so its own name is already
	// in scope at the call site) rather than re-reading `selectedChannel`
	// state here -- avoids a redundant "what if there's no channel" branch
	// this could otherwise never actually be reached without.
	function selectVersion(channel: string, ref: string): void {
		setSelectedVersion(ref);
		switchOnCore(channel, ref);
	}

	return {
		selectedChannel,
		selectedVersion,
		selectedChannelEntry,
		isPending: switchChannel.isPending,
		selectChannel,
		selectVersion,
	};
}
