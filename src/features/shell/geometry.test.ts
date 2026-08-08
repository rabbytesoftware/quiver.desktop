import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';

import { railOwnsControls, ROW_H, windowControls } from './geometry';

/**
 * Every case below states its platform through a real webview's UA string
 * rather than the host's, so a run on a macOS laptop and a run on a Linux CI
 * box assert the same table. Without the restore, the first test to call
 * `runningOn` decides the platform for every file that runs after it.
 */
afterEach(() => {
	restoreUserAgent();
});

describe('ROW_H', () => {
	it('is 34', () => {
		// `--row` in index.css and `trafficLightPosition.y` in
		// tauri.macos.conf.json are both derived from this number and neither
		// can import it. Moving it here alone leaves the traffic lights
		// off-centre in a row that no longer matches them.
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
		// A reserve here would be a 64px hole in the top row with nothing ever
		// painted into it — the buttons are up in the native title bar.
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
		// The one combination where the lights and the rail sit on opposite
		// edges. Answer `true` here and the reserve opens on the right, half a
		// window away from the buttons it is holding space for.
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
