import { useRef, useState } from 'react';

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
	//
	// `detail.channels` now arrives from its own slower query (`GET /v0/arrow/:ns/channels`,
	// arrow-details-screen.tsx) and can still be `[]` (not yet loaded) on the
	// very first seed for THIS SAME namespace -- the seed key folds in whether
	// the list has anything yet, so the moment it resolves (still the same
	// arrow, `detail.namespace` unchanged) this re-seeds exactly once more and
	// picks up the real default channel, rather than being stuck on whatever
	// `undefined`/`[]` produced before the fetch landed.
	const [seededFor, setSeededFor] = useState<string | null>(null);
	const [selectedChannel, setSelectedChannel] = useState<string | undefined>(undefined);
	const [selectedVersion, setSelectedVersion] = useState<string | undefined>(undefined);
	const seedKey = `${detail.namespace}#${detail.channels.length > 0}`;
	if (seededFor !== seedKey) {
		setSeededFor(seedKey);
		const initialChannel = detail.channel || detail.channels[0]?.name;
		setSelectedChannel(initialChannel);
		setSelectedVersion(defaultVersionOf(detail.channels.find((c) => c.name === initialChannel)));
	}
	const selectedChannelEntry = detail.channels.find((c) => c.name === selectedChannel);

	// `useMutation`'s own `isPending` only flips on React's NEXT render, not
	// synchronously within the click handler that started it -- leaving a gap
	// where two near-simultaneous clicks (e.g. the Channel select then the
	// Version select) can both read it as still `false` and both start a
	// switch before the first one's re-render ever lands. Confirmed live: a
	// genuine concurrent double-click on quiver.desktop's own Channel and
	// Version selects sent two overlapping PATCHes against the same starting
	// namespace -- the first's ref upgrade moved the server-side aggregate's
	// identity and forgot the old one, so the second (racing against a
	// namespace that had just stopped existing) came back a genuine 404. A
	// ref mutates immediately, with no such gap, so it -- not `isPending` --
	// is the actual concurrency guard; `isPending` remains a fine UX signal
	// for disabling the selects visually.
	const inFlight = useRef(false);

	function switchOnCore(channel: string, ref: string | undefined): void {
		if (!detail.user_installed) return;
		// Skip a switch the latest fetched `detail` already reflects -- a
		// harmless no-op guard, independent of the race above.
		if (channel === detail.channel && ref === detail.installed_ref) return;
		inFlight.current = true;
		switchChannel
			.mutateAsync({ namespace: detail.namespace, channel, ref })
			.then(() => queryClient.invalidateQueries({ queryKey: arrowDetailQueryKeyPrefix }))
			.catch(() => {})
			.finally(() => {
				inFlight.current = false;
			});
	}

	function selectChannel(name: string): void {
		// Checked before touching any local state: a pick that loses the race
		// must not partially land (neither its own core call nor its UI state)
		// alongside the switch already underway.
		if (inFlight.current) return;
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
		if (inFlight.current) return;
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
