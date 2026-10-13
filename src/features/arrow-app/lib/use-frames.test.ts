import { describe, expect, it } from 'vitest';

import { createFrameRegistry } from './use-frames';

describe('frame registry', () => {
	it('maps hosts to iframe windows and forgets removed frames', () => {
		const reg = createFrameRegistry();
		const win = {} as Window;
		reg.set('abc', { contentWindow: win } as HTMLIFrameElement);

		expect(reg.windowOf('abc')).toBe(win);
		expect(reg.hosts()).toEqual(['abc']);

		reg.set('abc', null);
		expect(reg.windowOf('abc')).toBeUndefined();
		expect(reg.hosts()).toEqual([]);
	});

	it('has no window for a frame that has not loaded one', () => {
		const reg = createFrameRegistry();
		reg.set('abc', { contentWindow: null } as HTMLIFrameElement);
		expect(reg.windowOf('abc')).toBeUndefined();
	});
});
