import { describe, expect, it } from 'vitest';

import { CircleHelpIcon } from 'lucide-react';

import { iconForStepType, isKnownStepType } from './step-type';

describe('step-type', () => {
	it.each(['run', 'fetch', 'extract', 'portable', 'signal', 'dependencies', 'expose', 'unexpose'])(
		'knows %s and gives it its own icon',
		(type) => {
			expect(isKnownStepType(type)).toBe(true);
			expect(iconForStepType(type)).not.toBe(CircleHelpIcon);
		}
	);

	it.each(['exec', 'toString', ''])('falls back to the generic icon for %j', (type) => {
		expect(isKnownStepType(type)).toBe(false);
		expect(iconForStepType(type)).toBe(CircleHelpIcon);
	});
});
