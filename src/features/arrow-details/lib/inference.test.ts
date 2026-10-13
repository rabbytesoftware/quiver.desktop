import { describe, expect, it } from 'vitest';

import { confidenceKey, warningKey } from './inference';

describe('warningKey', () => {
	it.each(['assumed_arch', 'emulated', 'windows_exe_unverified', 'name_mismatch', 'unpinned_rolling_tag'])(
		'maps %s to a message',
		(code) => {
			expect(warningKey(code)).toBe(`arrow.inferred.warning.${code}`);
		}
	);

	it.each(['brand_new_code', 'toString', ''])('has no message for %j', (code) => {
		expect(warningKey(code)).toBeNull();
	});
});

describe('confidenceKey', () => {
	it('maps the known tiers and nothing else', () => {
		expect(confidenceKey('high')).toBe('arrow.inferred.confidence.high');
		expect(confidenceKey('medium')).toBe('arrow.inferred.confidence.medium');
		expect(confidenceKey('low')).toBe('arrow.inferred.confidence.low');
		expect(confidenceKey('extreme')).toBeNull();
	});
});
