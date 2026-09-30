import { parseArrowOrigin, parseInferenceConfidence } from '@/domain/arrow';
import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import { getArrow, getArrowsFor, removeArrow, upsertArrow } from './entity-cache';
import type { ArrowCatalogRecord } from './schemas';

const ARROW_ENDPOINT = '/v0/arrow';

interface ArrowFrame {
	event: 'upserted' | 'removed';
	namespace: string;
	name?: string;
	description?: string;
	tags?: string[];
	icon?: string | null;
	banner?: string | null;
	media?: {
		icon?: string | null;
		banner?: string | null;
	};
	version?: string;
	user_installed?: boolean;
	origin?: string;
	inference?: { confidence?: string };
}

export interface SubscribeArrowStreamOptions {
	connectionId: string;
	seed: () => Promise<ArrowCatalogRecord[]>;
	onChange?: () => void;
	onSeedError?: (error: unknown) => void;
	/**
	 * Called with the namespace of an upsert frame that carries no version for
	 * a row the cache has none for either -- a row just registered, typically.
	 * quiver.core's frames never carry the resolved ref, so only a re-read of
	 * the catalog can supply it.
	 */
	onUnversionedUpsert?: (namespace: string) => void;
}

/** Disposes the stream when called; `reseed` re-reads the whole catalog the way a reconnect does, coalescing a burst of requests into at most one more read. */
export interface ArrowStream {
	(): void;
	reseed(): void;
}

export function subscribeArrowStream(opts: SubscribeArrowStreamOptions): ArrowStream {
	const { connectionId, seed, onChange, onSeedError, onUnversionedUpsert } = opts;
	let disposed = false;

	let applyChain: Promise<void> = Promise.resolve();
	let seedGeneration = 0;
	// Re-read requests coalesce: one seed in flight, at most one queued after it.
	let reseedPending = false;
	let reseedAgain = false;
	// Namespaces an unversioned frame asked a re-read for, and those a re-read
	// then showed are not library rows (dependency-only arrows, which the seed
	// never lists): asking again for those would re-read forever.
	const askedFor = new Set<string>();
	const unlisted = new Set<string>();

	async function applyFrame(frame: ArrowFrame): Promise<void> {
		if (frame.event === 'removed') {
			return removeArrow(connectionId, frame.namespace);
		}
		// quiver.core's catalog frames carry no version (the resolved ref is row
		// state, re-read from `GET /v0/arrow`), so a frame must not erase the
		// one the last seed recorded.
		const version = frame.version ?? (await getArrow(connectionId, frame.namespace))?.version ?? '';
		if (!version && frame.user_installed !== false && !unlisted.has(frame.namespace)) {
			askedFor.add(frame.namespace);
			onUnversionedUpsert?.(frame.namespace);
		}
		return upsertArrow({
			connectionId,
			namespace: frame.namespace,
			name: frame.name ?? '',
			description: frame.description ?? '',
			tags: frame.tags ?? [],
			icon: frame.media?.icon ?? frame.icon ?? null,
			banner: frame.media?.banner ?? frame.banner ?? null,
			version,
			origin: parseArrowOrigin(frame.origin),
			confidence: parseInferenceConfidence(frame.inference?.confidence),
		});
	}

	async function applySeed(generation: number): Promise<void> {
		const items = await seed();
		if (disposed || generation !== seedGeneration) return;
		const fresh = new Set(items.map((item) => item.namespace));
		for (const namespace of askedFor) {
			if (!fresh.has(namespace)) unlisted.add(namespace);
		}
		askedFor.clear();
		const cached = await getArrowsFor(connectionId);
		if (disposed || generation !== seedGeneration) return;
		const namespacesToPrune: string[] = [];
		for (const arrow of cached) {
			if (!fresh.has(arrow.namespace)) {
				namespacesToPrune.push(arrow.namespace);
			}
		}
		await Promise.all(namespacesToPrune.map((namespace) => removeArrow(connectionId, namespace)));
		await Promise.all(items.map((item) => upsertArrow(item)));
	}

	function runSeed(): Promise<void> {
		const generation = ++seedGeneration;
		applyChain = applyChain
			.then(() => applySeed(generation))
			.then(() => {
				if (!disposed) onChange?.();
			})
			// A rejection (a failed seed() GET) must NOT poison the chain: a .then
			// on a rejected promise skips every subsequent step, which would
			// permanently freeze this stream for the session (no live frames, no
			// reconnect reseed could recover). Absorb it here so applyChain is
			// always resolved for the next step; applySeed throws before mutating
			// the cache, so a failed reseed simply leaves the cache intact and a
			// later reseed retries.
			.catch((err: unknown) => {
				console.error(`entity-stream: seed failed for ${ARROW_ENDPOINT}`, err);
				if (!disposed) onSeedError?.(err);
			});
		return applyChain;
	}

	function requestReseed(): void {
		if (disposed) return;
		if (reseedPending) {
			reseedAgain = true;
			return;
		}
		reseedPending = true;
		void runSeed().then(() => {
			reseedPending = false;
			if (!reseedAgain) return;
			reseedAgain = false;
			requestReseed();
		});
	}

	void runSeed();

	const unsubscribe = wsManager.subscribe(ARROW_ENDPOINT, (data: unknown) => {
		if (disposed) return;
		if (isReconnectSentinel(data)) {
			void runSeed();
			return;
		}
		const frame = data as ArrowFrame;
		if (!frame || typeof frame.namespace !== 'string') return;
		applyChain = applyChain
			.then(() => applyFrame(frame))
			.then(() => {
				if (!disposed) onChange?.();
			})
			// applyFrame itself cannot reject — removeArrow/upsertArrow are
			// documented best-effort and swallow their own IDB errors. The
			// realistic source here is the caller's own onChange throwing (a
			// programming error, not a cache failure); absorb it so it can't
			// poison the chain and freeze all later frames + reseeds for the
			// session (see runSeed). Ordering is still preserved: a later frame
			// only runs after this one's catch resolves.
			.catch((err: unknown) => {
				console.error(`entity-stream: frame apply failed for ${ARROW_ENDPOINT}`, err);
			});
	});

	function dispose(): void {
		disposed = true;
		unsubscribe();
	}

	return Object.assign(dispose, {
		reseed: requestReseed,
	});
}
