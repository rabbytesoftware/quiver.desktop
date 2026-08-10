import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';

import { railOwnsControls, ROW_H, windowControls } from './geometry';

afterEach(() => {
	restoreUserAgent();
});

describe('ROW_H', () => {
	it('is 34', () => {
		expect(ROW_H).toBe(34);
	});
});

describe('windowControls', () => {
	it('reserves 64px on the left on macOS, where the OS paints the lights over our spacer', () => {
		runningOn(USER_AGENTS.macos);
		expect(windowControls()).toEqual({ edge: 'left', kind: 'reserve', width: 64 });
	});

	it.each([
		['Linux', USER_AGENTS.linux],
		['Windows', USER_AGENTS.windows],
	])('reserves nothing on %s, where the OS draws its own title bar', (_platform, userAgent) => {
		runningOn(userAgent);
		expect(windowControls()).toBeNull();
	});
});

describe('railOwnsControls', () => {
	it('gives the reserve to a left rail on macOS, which sits on the same edge as the lights', () => {
		runningOn(USER_AGENTS.macos);
		expect(railOwnsControls('left')).toBe(true);
	});

	it('withholds it from a right rail on macOS, leaving it to the chrome row', () => {
		runningOn(USER_AGENTS.macos);
		expect(railOwnsControls('right')).toBe(false);
	});

	it.each([
		['Linux', USER_AGENTS.linux],
		['Windows', USER_AGENTS.windows],
	])('gives it to neither side on %s, because there is nothing to place', (_platform, userAgent) => {
		runningOn(userAgent);
		expect(railOwnsControls('left')).toBe(false);
		expect(railOwnsControls('right')).toBe(false);
	});
});
