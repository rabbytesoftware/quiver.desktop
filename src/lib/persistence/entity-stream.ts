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
	last_used_at?: string;
	origin?: string;
	inference?: { confidence?: string } | null;
}

export interface SubscribeArrowStreamOptions {
	connectionId: string;
	seed: () => Promise<ArrowCatalogRecord[]>;
	onChange?: () => void;
	onSeedError?: (error: unknown) => void;
}

export function subscribeArrowStream(opts: SubscribeArrowStreamOptions): () => void {
	const { connectionId, seed, onChange, onSeedError } = opts;
	let disposed = false;

	let applyChain: Promise<void> = Promise.resolve();
	let seedGeneration = 0;

	async function applyFrame(frame: ArrowFrame): Promise<void> {
		if (frame.event === 'removed') {
			return removeArrow(connectionId, frame.namespace);
		}
		// A frame that omits origin/inference (an older core) must not erase what
		// the seed already recorded; a declared arrow never carries a confidence.
		const existing = await getArrow(connectionId, frame.namespace);
		const origin = frame.origin === undefined ? (existing?.origin ?? 'declared') : parseArrowOrigin(frame.origin);
		const confidence =
			origin === 'declared'
				? null
				: frame.inference?.confidence === undefined
					? (existing?.confidence ?? null)
					: parseInferenceConfidence(frame.inference.confidence);
		const lastUsedAt = frame.last_used_at ?? existing?.last_used_at;
		return upsertArrow({
			connectionId,
			namespace: frame.namespace,
			name: frame.name ?? '',
			description: frame.description ?? '',
			tags: frame.tags ?? [],
			icon: frame.media?.icon ?? frame.icon ?? null,
			banner: frame.media?.banner ?? frame.banner ?? null,
			version: frame.version ?? '',
			...(lastUsedAt ? { last_used_at: lastUsedAt } : {}),
			origin,
			confidence,
		});
	}

	async function applySeed(generation: number): Promise<void> {
		const items = await seed();
		if (disposed || generation !== seedGeneration) return;
		const fresh = new Set(items.map((item) => item.namespace));
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

	function runSeed(): void {
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
	}

	runSeed();

	const unsubscribe = wsManager.subscribe(ARROW_ENDPOINT, (data: unknown) => {
		if (disposed) return;
		if (isReconnectSentinel(data)) {
			runSeed();
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

	return () => {
		disposed = true;
		unsubscribe();
	};
}
