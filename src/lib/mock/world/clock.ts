import type { Clock } from './types';

/**
 * Nothing in the mock may call `setTimeout`/`setInterval` directly.
 *
 * A scenario switch, or a test finishing, ends a world while its timers are
 * still armed. Left running they mutate a world nobody reads and emit at closed
 * sockets — invisible in the app, cross-test contamination in the suite.
 * Funnelling them through one owner is what makes `cancelAll()` possible.
 */
export function createClock(): Clock {
	const timeouts = new Set<ReturnType<typeof setTimeout>>();
	const intervals = new Set<ReturnType<typeof setInterval>>();

	return {
		after(ms, fn) {
			const handle = setTimeout(() => {
				// Discarded before `fn` runs, because `fn` may arm another timer or
				// call `cancelAll`.
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
