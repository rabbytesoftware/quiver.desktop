import { describe, expect, it } from 'vitest';

import type { ArrowLifecycle, ArrowTarget } from './arrow';
import {
	isInferred,
	isPlatformSupported,
	parseArrowOrigin,
	parseInferenceConfidence,
	targetForPlatform,
} from './arrow';

const LIFECYCLE: ArrowLifecycle = {
	install: [],
	update: [],
	execute: [],
	stop: [],
	uninstall: [],
};

function target(platform: string): ArrowTarget {
	return { platform, requirement: { cpu_cores: 1, memory_gb: 1, disk_gb: 1 }, lifecycle: LIFECYCLE, methods: [] };
}

describe('isPlatformSupported', () => {
	it('is true when a target declares an exact match', () => {
		const targets = [target('linux/amd64'), target('darwin/arm64')];
		expect(isPlatformSupported(targets, 'darwin/arm64')).toBe(true);
	});

	it('is false when nothing matches, even though targetForPlatform would still fall back to the first entry', () => {
		const targets = [target('linux/amd64'), target('windows/amd64')];
		expect(isPlatformSupported(targets, 'darwin/arm64')).toBe(false);
		expect(targetForPlatform(targets, 'darwin/arm64')).toBe(targets[0]);
	});

	it('is false when there are no targets at all', () => {
		expect(isPlatformSupported([], 'darwin/arm64')).toBe(false);
	});

	it('is true for a single universal target that happens to match', () => {
		const targets = [target('linux/amd64')];
		expect(isPlatformSupported(targets, 'linux/amd64')).toBe(true);
	});
});

describe('parseArrowOrigin', () => {
	it.each([
		['inferred', 'inferred'],
		['declared', 'declared'],
		[undefined, 'declared'],
		[null, 'declared'],
		['', 'declared'],
		['something-newer', 'declared'],
	] as const)('reads %s as %s', (wire, expected) => {
		expect(parseArrowOrigin(wire)).toBe(expected);
	});
});

describe('parseInferenceConfidence', () => {
	it.each([
		['high', 'high'],
		['medium', 'medium'],
		['low', 'low'],
		[undefined, null],
		[null, null],
		['', null],
		['certain', null],
	] as const)('reads %s as %s', (wire, expected) => {
		expect(parseInferenceConfidence(wire)).toBe(expected);
	});
});

describe('isInferred', () => {
	it('is true only for an explicit inferred origin', () => {
		expect(isInferred({ origin: 'inferred' })).toBe(true);
		expect(isInferred({ origin: 'declared' })).toBe(false);
		expect(isInferred({})).toBe(false);
	});
});
