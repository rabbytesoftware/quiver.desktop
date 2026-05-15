import { describe, it, expect } from 'vitest';
import { localConnection, LOCAL_CONNECTION_ID } from './connection';

describe('localConnection', () => {
	it('returns correct id and kind', () => {
		const c = localConnection();
		expect(c.id).toBe(LOCAL_CONNECTION_ID);
		expect(c.kind).toBe('local');
		expect(c.url).toBeUndefined();
		expect(c.api_version).toBe('v0');
	});
});
