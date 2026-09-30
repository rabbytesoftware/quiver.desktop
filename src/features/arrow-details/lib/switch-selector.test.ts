import { describe, expect, it, vi } from 'vitest';

import type { ArrowState } from '@/domain/arrow';

import { canSwitch, switchSelector, type SwitchSteps } from './switch-selector';

function recordingSteps(): { steps: SwitchSteps; calls: string[] } {
	const calls: string[] = [];
	const step = (name: string) =>
		vi.fn((ns: string) => {
			calls.push(`${name} ${ns}`);
			return Promise.resolve();
		});
	return {
		calls,
		steps: {
			uninstall: step('uninstall'),
			remove: step('remove'),
			register: step('register'),
			install: step('install'),
		},
	};
}

const FROM = 'github.com/char2cs/crowbar@stable';
const TO = 'github.com/char2cs/crowbar@beta';

describe('switchSelector', () => {
	it('uninstalls, forgets the old row, then registers and installs the new identity -- in that order', async () => {
		const { steps, calls } = recordingSteps();
		await switchSelector({ from: FROM, to: TO, installed: true }, steps);
		expect(calls).toEqual([`uninstall ${FROM}`, `remove ${FROM}`, `register ${TO}`, `install ${TO}`]);
	});

	it('only swaps catalog rows when nothing was on disk', async () => {
		const { steps, calls } = recordingSteps();
		await switchSelector({ from: FROM, to: TO, installed: false }, steps);
		expect(calls).toEqual([`remove ${FROM}`, `register ${TO}`]);
	});

	it('stops at the first step that fails, leaving the rest undone', async () => {
		const { steps, calls } = recordingSteps();
		steps.uninstall = vi.fn(() => Promise.reject(new Error('uninstall failed')));
		await expect(switchSelector({ from: FROM, to: TO, installed: true }, steps)).rejects.toThrow(
			'uninstall failed'
		);
		expect(calls).toEqual([]);
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
