import type { Clock } from './types';

export function createClock(): Clock {
	const timeouts = new Set<ReturnType<typeof setTimeout>>();
	const intervals = new Set<ReturnType<typeof setInterval>>();

	return {
		after(ms, fn) {
			const handle = setTimeout(() => {
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
