import type { Clock } from './types';

/**
 * Every fabricated transition runs on one of these, and nothing else in the
 * mock is allowed to call `setTimeout`/`setInterval` directly.
 *
 * The reason is teardown. An install is a repeating timer that mutates the
 * world and pushes frames at the socket hub; a scenario switch, an uninstall
 * landing mid-install, or a test finishing all end that world's life while the
 * timer is still armed. Left running it writes into a world nobody reads and
 * emits at sockets that have closed — invisible in the app, and cross-test
 * contamination in the suite. Funnelling them through one owner makes
 * `cancelAll()` possible at all.
 */
export function createClock(): Clock {
	const timeouts = new Set<ReturnType<typeof setTimeout>>();
	const intervals = new Set<ReturnType<typeof setInterval>>();

	return {
		after(ms, fn) {
			const handle = setTimeout(() => {
				// Discarded before the callback runs, not after: `fn` may itself arm
				// another timer or call `cancelAll`, and a handle still sitting in
				// the set at that point would be cleared twice or — worse — leak,
				// since a fired timeout can never be cleared meaningfully again.
				timeouts.delete(handle);
				fn();
			}, ms);
			timeouts.add(handle);
		},

		every(ms, fn) {
			const handle = setInterval(fn, ms);
			intervals.add(handle);
			return () => {
				clearInterval(handle);
				intervals.delete(handle);
			};
		},

		cancelAll() {
			timeouts.forEach(clearTimeout);
			intervals.forEach(clearInterval);
			timeouts.clear();
			intervals.clear();
		},
	};
}
