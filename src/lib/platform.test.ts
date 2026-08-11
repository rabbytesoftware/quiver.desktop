import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';

import { isMacOS } from './platform';

describe('isMacOS', () => {
	afterEach(() => {
		restoreUserAgent();
	});

	it('recognises the macOS webview', () => {
		runningOn(USER_AGENTS.macos);
		expect(isMacOS()).toBe(true);
	});

	it('does not mistake the Linux webview for it', () => {
		runningOn(USER_AGENTS.linux);
		expect(isMacOS()).toBe(false);
	});

	it('does not mistake the Windows webview for it', () => {
		runningOn(USER_AGENTS.windows);
		expect(isMacOS()).toBe(false);
	});
});
