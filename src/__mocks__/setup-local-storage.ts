import { JSDOM } from 'jsdom';

/**
 * Suite-wide fix, not a design concern of any one test: some Node/Bun
 * versions ship a built-in, incomplete global `localStorage` (an unfinished
 * Web Storage API — present, but missing standard methods such as
 * `.clear()`). Vitest's own jsdom test environment refuses to copy jsdom's
 * *real* `localStorage` onto `globalThis` whenever a same-named property
 * already exists there (see `getWindowKeys` in vitest's
 * `dist/chunks/index.*.js`: any jsdom window key already `in global` is
 * skipped unless it's in vitest's hardcoded allowlist, and
 * `localStorage`/`sessionStorage` are not on it) — so the broken built-in
 * wins, and `localStorage.clear()` throws `TypeError: ... is not a function`
 * in any test that touches it.
 *
 * This does NOT reach into vitest's internal environment handle
 * (`globalThis.jsdom`) to steal its Storage instance. Instead it builds an
 * independent JSDOM instance via jsdom's own public `JSDOM` export — already
 * a project devDependency — purely to source a real, spec-compliant
 * `Storage`, and installs it in place of whatever `globalThis.localStorage`
 * happens to be. That keeps this file decoupled from any Vitest-version
 * implementation detail: it has nothing to break if a Vitest upgrade
 * restructures or removes the environment's internal handle.
 *
 * `url` matches vitest's own jsdom environment default (`http://localhost:3000`)
 * so this Storage is same-origin with the rest of the DOM globals vitest
 * installs; jsdom refuses `localStorage` for opaque/`about:blank` origins.
 */
const { window } = new JSDOM('', { url: 'http://localhost:3000' });

Object.defineProperty(globalThis, 'localStorage', {
	value: window.localStorage,
	configurable: true,
});
