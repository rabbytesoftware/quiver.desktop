import { afterEach, describe, expect, it } from 'vitest';

import { restoreUserAgent, runningOn, USER_AGENTS } from '@/__mocks__/user-agent';

import { currentPlatform, isMacOS } from './platform';

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

describe('currentPlatform', () => {
	afterEach(() => {
		restoreUserAgent();
	});

	it('reports darwin/amd64 for an Intel-shaped macOS user agent', () => {
		runningOn(USER_AGENTS.macos);
		expect(currentPlatform()).toBe('darwin/amd64');
	});

	it('reports linux/amd64 for the Linux webview', () => {
		runningOn(USER_AGENTS.linux);
		expect(currentPlatform()).toBe('linux/amd64');
	});

	it('reports windows/amd64 for the Windows webview', () => {
		runningOn(USER_AGENTS.windows);
		expect(currentPlatform()).toBe('windows/amd64');
	});

	it('reports darwin/arm64 when the user agent carries an arm64 hint', () => {
		runningOn('Mozilla/5.0 (Macintosh; ARM64 Mac OS X 14_0) AppleWebKit/605.1.15');
		expect(currentPlatform()).toBe('darwin/arm64');
	});

	it('falls back to linux when neither macOS nor Windows is recognized', () => {
		runningOn('Mozilla/5.0 (SomeOtherOS)');
		expect(currentPlatform()).toBe('linux/amd64');
	});
});
