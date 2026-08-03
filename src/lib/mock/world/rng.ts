/**
 * mulberry32 — a small, seeded PRNG.
 *
 * Nothing in a scenario may call `Math.random()`. Two reasons, and the second
 * is the one that bites: screenshots taken on two machines would disagree, and
 * `buildWorld('extreme')` could not be asserted deep-equal to itself, which is
 * the only cheap way to catch a scenario quietly acquiring a random source
 * later. Seeding makes "generated" and "fixed" the same thing.
 *
 * Chosen over `crypto.getRandomValues` precisely because it is reproducible,
 * and over a hand-rolled LCG because those have short cycles in the low bits —
 * visible here as, say, every eighth generated arrow landing on the same state.
 */
export function createRng(seed: number): () => number {
	let a = seed >>> 0;
	return () => {
		a = (a + 0x6d2b79f5) >>> 0;
		let t = a;
		t = Math.imul(t ^ (t >>> 15), t | 1);
		t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

/** Uniform pick. Never called with an empty list in this codebase. */
export function pick<T>(rng: () => number, items: readonly T[]): T {
	return items[Math.floor(rng() * items.length)];
}

export function intBetween(rng: () => number, min: number, max: number): number {
	return min + Math.floor(rng() * (max - min + 1));
}
