// `jsdom` ships no type declarations of its own, and there is no
// `@types/jsdom` package installed — adding one would be a new
// `package.json` dependency outside this task's sanctioned change (`idb` +
// `fake-indexeddb` only). This declares only the tiny slice of jsdom's public
// API that `setup-local-storage.ts` actually uses, so that file can import
// the real `jsdom` package (already a devDependency, used internally by
// Vitest's own jsdom test environment) without reaching into any Vitest
// internal to get there.
declare module 'jsdom' {
	export class JSDOM {
		constructor(html?: string, options?: { url?: string });
		readonly window: { localStorage: Storage };
	}
}
