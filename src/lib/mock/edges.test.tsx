// The fallbacks and defaults — the paths that only run when something has
// already gone slightly wrong, and therefore the paths nothing else exercises.

import { describe, expect, it, vi } from 'vitest';

import { Slider } from '@/components/ui/controls';

import { useSettingsUI } from '@/features/settings/store';

import { shouldFault } from './server/chaos';
import { useMockStore } from './store';
import { buildWorld, providersFor } from './world/build';
import { createClock } from './world/clock';
import { createRng, intBetween, pick } from './world/rng';
import { getScenario, SCENARIOS } from './world/scenarios';

const noEmitter = { emit: () => {} };

describe('getScenario', () => {
	it('returns the descriptor it was asked for', () => {
		expect(getScenario('extreme').label).toBe('Extreme');
	});

	// A stale `quiver.mock` naming a scenario that no longer exists must not be
	// able to brick a release build for someone who cannot reach the setting.
	it('falls back to the first scenario rather than throwing on an unknown name', () => {
		expect(getScenario('rabbyte-only-2024')).toBe(SCENARIOS[0]);
	});

	it('gives every scenario a label and a summary worth showing', () => {
		for (const scenario of SCENARIOS) {
			expect(scenario.label.length).toBeGreaterThan(0);
			expect(scenario.summary.length).toBeGreaterThan(0);
		}
	});
});

describe('the world builder', () => {
	it('numbers ids monotonically rather than randomly, so runs are comparable', () => {
		const world = buildWorld('empty', noEmitter);
		expect([world.nextId(), world.nextId(), world.nextId()]).toEqual([1, 2, 3]);
		world.clock.cancelAll();
	});

	it('hands back the providers for the scenario, empty included', () => {
		expect(providersFor('normal').some((p) => !p.ok)).toBe(true);
		expect(providersFor('empty')).toEqual([]);
	});
});

describe('the clock', () => {
	it('cancels a pending timeout that has not fired', () => {
		vi.useFakeTimers();
		const clock = createClock();
		const fn = vi.fn();

		clock.after(500, fn);
		clock.cancelAll();
		vi.advanceTimersByTime(2000);

		expect(fn).not.toHaveBeenCalled();
		vi.useRealTimers();
	});

	it('lets an interval be stopped by its own returned handle', () => {
		vi.useFakeTimers();
		const clock = createClock();
		const fn = vi.fn();

		const stop = clock.every(100, fn);
		vi.advanceTimersByTime(250);
		stop();
		vi.advanceTimersByTime(1000);

		expect(fn).toHaveBeenCalledTimes(2);
		clock.cancelAll();
		vi.useRealTimers();
	});

	// A fired timeout can never be cleared meaningfully again; it has to leave
	// the set before its callback runs, because that callback may arm another.
	it('forgets a timeout before running it, so re-arming from inside is safe', () => {
		vi.useFakeTimers();
		const clock = createClock();
		const seen: number[] = [];

		clock.after(10, () => {
			seen.push(1);
			clock.after(10, () => seen.push(2));
		});
		vi.advanceTimersByTime(100);

		expect(seen).toEqual([1, 2]);
		clock.cancelAll();
		vi.useRealTimers();
	});
});

describe('the seeded rng', () => {
	it('produces the same stream for the same seed, and a different one otherwise', () => {
		const a = createRng(42);
		const b = createRng(42);
		const c = createRng(43);
		const draw = (rng: () => number) => [rng(), rng(), rng()];

		expect(draw(a)).toEqual(draw(b));
		expect(draw(createRng(42))).not.toEqual(draw(c));
	});

	it('stays inside [0, 1)', () => {
		const rng = createRng(7);
		for (let i = 0; i < 500; i++) {
			const n = rng();
			expect(n).toBeGreaterThanOrEqual(0);
			expect(n).toBeLessThan(1);
		}
	});

	it('picks within the list and the range', () => {
		const rng = createRng(9);
		for (let i = 0; i < 100; i++) {
			expect(['a', 'b', 'c']).toContain(pick(rng, ['a', 'b', 'c']));
			const n = intBetween(rng, 3, 6);
			expect(n).toBeGreaterThanOrEqual(3);
			expect(n).toBeLessThanOrEqual(6);
		}
	});
});

describe('shouldFault', () => {
	it('never fires at zero, however unlucky the draw', () => {
		useMockStore.getState().resetFaults();
		expect(shouldFault('arrows', () => 0)).toBe(false);
	});

	it('always fires at a hundred, however lucky the draw', () => {
		useMockStore.getState().setFault('arrows', 100);
		expect(shouldFault('arrows', () => 0.999)).toBe(true);
		useMockStore.getState().resetFaults();
	});

	// The improvement over the reference implementation, which calls
	// Math.random() inline and so cannot be tested at all.
	it('honours an injected rng rather than reaching for Math.random', () => {
		useMockStore.getState().setFault('search', 50);
		expect(shouldFault('search', () => 0.49)).toBe(true);
		expect(shouldFault('search', () => 0.51)).toBe(false);
		useMockStore.getState().resetFaults();
	});

	it('reads the live store when no settings are passed', () => {
		useMockStore.getState().setFault('runtime', 100);
		expect(shouldFault('runtime', () => 0.5)).toBe(true);
		useMockStore.getState().resetFaults();
		expect(shouldFault('runtime', () => 0.5)).toBe(false);
	});
});

describe('the settings ui store', () => {
	it('opens on the current tab when none is named', () => {
		useSettingsUI.setState({ open: false, tab: 'developer', query: '' });
		useSettingsUI.getState().openSettings();
		expect(useSettingsUI.getState()).toMatchObject({ open: true, tab: 'developer' });
	});

	it('switches tabs without closing', () => {
		useSettingsUI.getState().setTab('connections');
		expect(useSettingsUI.getState()).toMatchObject({ open: true, tab: 'connections' });
	});
});

describe('the Slider wrapper', () => {
	// Base UI hands back an ARRAY for range sliders and a bare number for single
	// ones. Every caller here wants the number — a slider reporting `[15]` would
	// be stored as a fault percentage of `NaN`, and that route would then never
	// fault at all, silently.
	it('reports a number, not the array Base UI can hand back', async () => {
		const { render, screen } = await import('@testing-library/react');
		const userEvent = (await import('@testing-library/user-event')).default;
		const user = userEvent.setup();
		const seen: unknown[] = [];

		render(<Slider value={10} step={5} onValueChange={(v) => seen.push(v)} aria-label="probe" />);

		// Focused, not clicked. A click starts Base UI's pointer-capture drag,
		// and jsdom implements no `setPointerCapture` — the resulting unhandled
		// TypeError fails the whole run while every assertion still passes, which
		// is a uniquely unhelpful way to break a suite.
		screen.getByRole('slider', { name: 'probe' }).focus();
		await user.keyboard('{ArrowRight}');

		expect(seen.length).toBeGreaterThan(0);
		expect(typeof seen[0]).toBe('number');
		expect(seen[0]).toBe(15);
	});
});
