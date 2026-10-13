import { describe, expect, it } from 'vitest';

import { isQuiverOwnComponent, QUIVER_CORE_NAMESPACE, QUIVER_DESKTOP_NAMESPACE } from './release';

describe('isQuiverOwnComponent', () => {
	it('matches quiver.desktop, bare', () => {
		expect(isQuiverOwnComponent(QUIVER_DESKTOP_NAMESPACE)).toBe(true);
	});

	it('matches quiver.desktop, with a ref', () => {
		expect(isQuiverOwnComponent(`${QUIVER_DESKTOP_NAMESPACE}@stable-1.0`)).toBe(true);
	});

	it('matches quiver.core, bare', () => {
		expect(isQuiverOwnComponent(QUIVER_CORE_NAMESPACE)).toBe(true);
	});

	it('matches quiver.core, with a ref', () => {
		expect(isQuiverOwnComponent(`${QUIVER_CORE_NAMESPACE}@v1.0.0`)).toBe(true);
	});

	it('does not match an unrelated namespace', () => {
		expect(isQuiverOwnComponent('github.com/rabbyte/minecraft@v1.21.4')).toBe(false);
	});

	it('does not match a namespace that merely contains one of the two as a substring', () => {
		expect(isQuiverOwnComponent('github.com/rabbytesoftware/quiver.core-extra')).toBe(false);
	});
});
