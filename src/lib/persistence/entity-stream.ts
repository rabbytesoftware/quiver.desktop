import { isReconnectSentinel, wsManager } from '@/lib/transport/ws-manager';

import { getArrowsFor, removeArrow, upsertArrow } from './entity-cache';
import type { ArrowCatalogRecord } from './schemas';

// Live-cache core: subscribe the arrow WS endpoint and MERGE each frame into
// the arrow cache (event:'removed' → tombstone), rather than refetching the
// whole catalog on every delta.
//
// Startup: run seed() (a GET) and upsert all results, so the cache is correct
// before the first push arrives. On the reconnect sentinel ({reconnected:true})
// re-run seed() — a full GET reseed — because frames missed during the outage
// can never be recovered from a single DTO merge.

const ARROW_ENDPOINT = '/v0/arrow';

/**
 * Wire shape of a `/v0/arrow` WS frame. Only `event` and `namespace` are
 * guaranteed — the rest of the catalog fields ride along on an `'upserted'`
 * frame and are absent on a `'removed'` tombstone.
 */
interface ArrowFrame {
	event: 'upserted' | 'removed';
	namespace: string;
	name?: string;
	description?: string;
	tags?: string[];
	icon?: string | null;
	banner?: string | null;
	version?: string;
}

export interface SubscribeArrowStreamOptions {
	/** Which backend connection this stream's rows belong to. */
	connectionId: string;
	/** One-time GET (`/v0/arrow`) that returns the full current catalog. */
	seed: () => Promise<ArrowCatalogRecord[]>;
	/** Called after every cache mutation (seed batch, upsert, remove). */
	onChange?: () => void;
}

export function subscribeArrowStream(opts: SubscribeArrowStreamOptions): () => void {
	const { connectionId, seed, onChange } = opts;
	let disposed = false;

	// Ordering: every cache mutation (seed + each live frame) is queued onto a
	// single serial promise chain so writes commit in ARRIVAL ORDER. Without
	// this each frame ran its own fire-and-forget async IDB transaction, so an
	// 'upserted' then 'removed' pair for the same namespace could race — if the
	// delete committed first, the late upsert resurrected a tombstoned ghost
	// row (H21). Chaining a later delete strictly after an earlier upsert
	// removes that window.
	let applyChain: Promise<void> = Promise.resolve();
	// Each reseed bumps the generation; an in-flight seed whose generation is no
	// longer current has been superseded by a newer reconnect reseed and must
	// not write stale rows.
	let seedGeneration = 0;

	function applyFrame(frame: ArrowFrame): Promise<void> {
		if (frame.event === 'removed') {
			return removeArrow(connectionId, frame.namespace);
		}
		return upsertArrow({ ...frame, connectionId } as unknown as ArrowCatalogRecord);
	}

	// Authoritative + atomic reseed: a GET is only a point-in-time snapshot, so
	// upserting its rows is not enough — arrows deleted during the outage would
	// linger. We diff the fresh set against the cache and removeArrow any
	// namespace no longer present, then upsert the fresh rows, pruning ghosts
	// left by missed delete frames. Folding this into applyChain (below) keeps
	// it from interleaving with live frames.
	//
	// getArrowsFor(connectionId) is the structural partition that stands in for
	// Crowbar's pruneScope: it is an indexed read keyed on connectionId, so this
	// seed can only ever see — and therefore only ever prune — its own
	// connection's rows. There is no generic store shared across connections to
	// accidentally wipe.
	async function applySeed(generation: number): Promise<void> {
		const items = await seed();
		// A newer reseed started while our GET was in flight; let it win.
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
			});
	}

	// Seed before the first push lands; the WS subscription is registered
	// synchronously below so no frame is dropped while the GET is in flight.
	runSeed();

	const unsubscribe = wsManager.subscribe(ARROW_ENDPOINT, (data: unknown) => {
		if (disposed) return;
		// The reconnect sentinel is NOT a DTO — never upsert it; trigger a full
		// GET reseed so any frames missed during the outage are recovered.
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
			// Absorb a failed frame apply so it can't poison the chain and freeze
			// all later frames + reseeds for the session (see runSeed). Ordering is
			// still preserved: a later frame only runs after this one's catch
			// resolves.
			.catch((err: unknown) => {
				console.error(`entity-stream: frame apply failed for ${ARROW_ENDPOINT}`, err);
			});
	});

	return () => {
		disposed = true;
		unsubscribe();
	};
}
