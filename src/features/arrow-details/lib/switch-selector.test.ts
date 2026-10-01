import { describe, expect, it, vi } from 'vitest';

import type { ArrowState } from '@/domain/arrow';

import { canSwitch, SwitchError, switchSelector, type SwitchStep, type SwitchSteps } from './switch-selector';

function recordingSteps(failAt?: SwitchStep): { steps: SwitchSteps; calls: string[]; registered: () => void } {
	const calls: string[] = [];
	const step = (name: SwitchStep) =>
		vi.fn((ns: string) => {
			if (name === failAt) return Promise.reject(new Error(`${name} failed`));
			calls.push(`${name} ${ns}`);
			return Promise.resolve();
		});
	const registered = vi.fn();
	return {
		calls,
		registered,
		steps: {
			register: step('register'),
			uninstall: step('uninstall'),
			remove: step('remove'),
			install: step('install'),
			registered,
		},
	};
}

const FROM = 'github.com/char2cs/crowbar@stable';
const TO = 'github.com/char2cs/crowbar@beta';

describe('switchSelector', () => {
	it('registers the new identity first, then uninstalls and forgets the old row, then installs the new one', async () => {
		const { steps, calls, registered } = recordingSteps();
		await switchSelector({ from: FROM, to: TO, installed: true }, steps);
		expect(calls).toEqual([`register ${TO}`, `uninstall ${FROM}`, `remove ${FROM}`, `install ${TO}`]);
		expect(registered).toHaveBeenCalledWith(TO);
	});

	it('only swaps the catalog rows when nothing was on disk', async () => {
		const { steps, calls } = recordingSteps();
		await switchSelector({ from: FROM, to: TO, installed: false }, steps);
		expect(calls).toEqual([`register ${TO}`, `remove ${FROM}`]);
	});

	it.each<[SwitchStep, string[], boolean]>([
		['register', [], false],
		['uninstall', [`register ${TO}`], true],
		['remove', [`register ${TO}`, `uninstall ${FROM}`], true],
		['install', [`register ${TO}`, `uninstall ${FROM}`, `remove ${FROM}`], true],
	])(
		'stops at a failed %s, naming the step, with only the steps before it done',
		async (failAt, done, wasRegistered) => {
			const { steps, calls, registered } = recordingSteps(failAt);
			const err = await switchSelector({ from: FROM, to: TO, installed: true }, steps).catch((e: unknown) => e);
			expect(err).toBeInstanceOf(SwitchError);
			expect(err).toMatchObject({ step: failAt, reason: `${failAt} failed` });
			expect(calls).toEqual(done);
			expect(registered).toHaveBeenCalledTimes(wasRegistered ? 1 : 0);
		}
	);

	it('keeps a non-Error rejection’s text as the reason', async () => {
		const { steps } = recordingSteps();
		steps.register = vi.fn(() => Promise.reject('offline'));
		await expect(switchSelector({ from: FROM, to: TO, installed: false }, steps)).rejects.toMatchObject({
			step: 'register',
			reason: 'offline',
		});
	});
});

describe('canSwitch', () => {
	it.each<[ArrowState, boolean]>([
		['absent', true],
		['ready', true],
		['outdated', true],
		['running', false],
		['installing', false],
		['updating', false],
		['uninstalling', false],
		['detached', false],
	])('%s -> %s', (state, expected) => {
		expect(canSwitch(state)).toBe(expected);
	});
});
