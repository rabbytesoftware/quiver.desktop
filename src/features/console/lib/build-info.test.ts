import { beforeAll, describe, expect, it, vi } from 'vitest';

import {
	channelOfLabel,
	describeBuildLong,
	describeCore,
	describeDesktop,
	formatBuild,
	formatBuildTime,
	normaliseChannel,
	parseVersion,
	type BuildDescriptor,
} from './build-info';

// 2026-10-04T14:02:09Z
const BUILT = 1_791_122_529;
const SHA = '7b4dc02aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa';

function build(over: Partial<BuildDescriptor>): BuildDescriptor {
	return { channel: 'nightly', version: null, commit: SHA, builtAt: BUILT, ...over };
}

beforeAll(() => {
	// The timestamp is shown in the viewer's zone; pin it so the expectations are literals.
	vi.stubEnv('TZ', 'UTC');
});

describe('normaliseChannel', () => {
	it.each([
		['stable', 'stable'],
		['beta', 'beta'],
		['hotfix', 'hotfix'],
		['nightly', 'nightly'],
		['nightly-rolling', 'nightly'],
		['nightly-latest', 'nightly'],
		[' Beta ', 'beta'],
	])('%s → %s', (raw, want) => expect(normaliseChannel(raw)).toBe(want));

	it.each(['', '  ', 'develop', 'canary', undefined, null])('%s is not a channel', (raw) =>
		expect(normaliseChannel(raw)).toBeNull()
	);
});

describe('channelOfLabel', () => {
	it('reads the channel off a release tag', () => {
		expect(channelOfLabel('stable-26.5.1')).toBe('stable');
		expect(channelOfLabel('beta-26.5-4')).toBe('beta');
		expect(channelOfLabel('hotfix-26.5.1-2')).toBe('hotfix');
	});

	it('refuses anything else', () => {
		for (const bad of ['nightly-rolling', '26.5.1', 'develop', '', null, undefined]) {
			expect(channelOfLabel(bad)).toBeNull();
		}
	});
});

describe('parseVersion', () => {
	it.each([
		['stable-26.5.1', { series: '26.5.1', build: null }],
		['stable-26.5', { series: '26.5', build: null }],
		['beta-26.5-4', { series: '26.5', build: 4 }],
		['26.5-4', { series: '26.5', build: 4 }],
		['26.5.1', { series: '26.5.1', build: null }],
		['beta-2026-09-27', { series: '2026-09-27', build: null }],
		['beta-2026-09-27-1', { series: '2026-09-27', build: 1 }],
		['hotfix-26.5.1-2', { series: '26.5.1', build: 2 }],
	])('%s', (raw, want) => expect(parseVersion(raw)).toEqual(want));

	it.each(['', 'dev', 'beta-', 'beta-26', 'beta-26.5-x', 'v26.5', null, undefined])('%s is not a version', (raw) =>
		expect(parseVersion(raw)).toBeNull()
	);
});

describe('formatBuildTime', () => {
	it('is month-day and time', () => expect(formatBuildTime(BUILT)).toBe('10-04 14:02'));
	it('drops the time when compact', () => expect(formatBuildTime(BUILT, true)).toBe('10-04'));
	it('pads single digits', () => expect(formatBuildTime(Date.UTC(2026, 0, 2, 3, 4) / 1000)).toBe('01-02 03:04'));
});

describe('formatBuild at rest', () => {
	it('shows a rolling build by its timestamp', () => {
		expect(formatBuild(build({ channel: 'nightly' }))).toBe('nightly 10-04 14:02');
	});

	it('shows only the channel for a rolling build with no timestamp', () => {
		expect(formatBuild(build({ channel: 'nightly', builtAt: null }))).toBe('nightly');
	});

	it('shows a stable release by its version', () => {
		expect(formatBuild(build({ channel: 'stable', version: 'stable-26.5.1' }))).toBe('stable 26.5.1');
		expect(formatBuild(build({ channel: 'stable', version: '26.5' }))).toBe('stable 26.5');
	});

	it('shows a beta release by version and build count', () => {
		expect(formatBuild(build({ channel: 'beta', version: 'beta-26.5-4' }))).toBe('beta 26.5 #4');
		expect(formatBuild(build({ channel: 'beta', version: 'beta-26.5' }))).toBe('beta 26.5');
		expect(formatBuild(build({ channel: 'beta', version: 'beta-2026-09-27-1' }))).toBe('beta 2026-09-27 #1');
	});

	it('shows a hotfix the same way', () => {
		expect(formatBuild(build({ channel: 'hotfix', version: 'hotfix-26.5.1-2' }))).toBe('hotfix 26.5.1 #2');
	});

	it('shows a release timestamp never: a release is identified by its version', () => {
		expect(formatBuild(build({ channel: 'stable', version: 'stable-26.5.1' }))).not.toContain('10-04');
	});

	it('says only the channel when a release has no version', () => {
		expect(formatBuild(build({ channel: 'beta', version: null }))).toBe('beta');
	});

	it('keeps an unparseable version rather than inventing one', () => {
		expect(formatBuild(build({ channel: 'stable', version: 'weird' }))).toBe('stable weird');
	});

	it('says dev for an unstamped build, whatever else it knows', () => {
		expect(formatBuild(build({ channel: 'dev' }))).toBe('dev');
		expect(formatBuild({ channel: 'dev', version: null, commit: null, builtAt: null })).toBe('dev');
	});
});

describe('formatBuild compact', () => {
	it('drops the time of day from a rolling build', () => {
		expect(formatBuild(build({ channel: 'nightly' }), { compact: true })).toBe('nightly 10-04');
	});

	it('leaves releases alone: they are already short', () => {
		expect(formatBuild(build({ channel: 'beta', version: 'beta-26.5-4' }), { compact: true })).toBe('beta 26.5 #4');
	});
});

describe('formatBuild on hover', () => {
	it('swaps in the short commit for every channel', () => {
		expect(formatBuild(build({ channel: 'nightly' }), { hover: true })).toBe('nightly 7b4dc02');
		expect(formatBuild(build({ channel: 'stable', version: 'stable-26.5.1' }), { hover: true })).toBe(
			'stable 7b4dc02'
		);
		expect(formatBuild(build({ channel: 'beta', version: 'beta-26.5-4' }), { hover: true })).toBe('beta 7b4dc02');
		expect(formatBuild(build({ channel: 'dev' }), { hover: true })).toBe('dev 7b4dc02');
	});

	it('accepts a commit that is already short', () => {
		expect(formatBuild(build({ commit: '7b4dc02' }), { hover: true })).toBe('nightly 7b4dc02');
	});

	it('falls back to the resting text when there is no commit', () => {
		expect(formatBuild(build({ commit: null }), { hover: true })).toBe('nightly 10-04 14:02');
		expect(formatBuild(build({ channel: 'dev', commit: '' }), { hover: true })).toBe('dev');
	});

	it('is not narrowed by compact', () => {
		expect(formatBuild(build({}), { hover: true, compact: true })).toBe('nightly 7b4dc02');
	});
});

describe('describeDesktop', () => {
	const base = { label: null, commit: SHA, builtAt: BUILT };

	it('takes the channel from the build channel', () => {
		expect(describeDesktop({ ...base, channel: 'nightly-rolling' }).channel).toBe('nightly');
		expect(describeDesktop({ ...base, channel: 'beta', label: 'beta-26.5-2' })).toEqual({
			channel: 'beta',
			version: 'beta-26.5-2',
			commit: SHA,
			builtAt: BUILT,
		});
	});

	it('falls back to the channel of the release label', () => {
		expect(describeDesktop({ ...base, channel: '', label: 'stable-26.5' }).channel).toBe('stable');
	});

	it('is dev with neither a channel nor a label', () => {
		expect(describeDesktop({ ...base, channel: '' }).channel).toBe('dev');
		expect(describeDesktop({ channel: '  ', label: null, commit: null, builtAt: null })).toEqual({
			channel: 'dev',
			version: null,
			commit: null,
			builtAt: null,
		});
	});

	it('trusts the pipeline channel over a label that disagrees', () => {
		expect(describeDesktop({ ...base, channel: 'hotfix', label: 'beta-26.5-1' }).channel).toBe('hotfix');
	});
});

describe('describeCore', () => {
	it('is null until the daemon has answered', () => {
		expect(describeCore(null)).toBeNull();
	});

	it('reads /versions', () => {
		expect(
			describeCore({ version: 'stable-26.5.1', commit: SHA, builtAt: '2026-10-04T14:02:09Z', channel: 'stable' })
		).toEqual({ channel: 'stable', version: 'stable-26.5.1', commit: SHA, builtAt: BUILT });
	});

	it('reads the rolling channel the way the pipeline names it', () => {
		const core = describeCore({
			version: 'nightly-latest',
			commit: SHA,
			builtAt: '2026-10-04T14:02:09Z',
			channel: 'nightly-latest',
		});
		expect(core?.channel).toBe('nightly');
		expect(formatBuild(core!)).toBe('nightly 10-04 14:02');
		expect(formatBuild(core!, { hover: true })).toBe('nightly 7b4dc02');
	});

	it('is dev when the daemon reports no channel and a version that is no release', () => {
		const core = describeCore({ version: 'dev', commit: '', builtAt: '', channel: '' });
		expect(core).toEqual({ channel: 'dev', version: 'dev', commit: null, builtAt: null });
		expect(formatBuild(core!)).toBe('dev');
	});

	it('derives the channel from a release version when the daemon omits it', () => {
		expect(describeCore({ version: 'beta-26.5-4', commit: '', builtAt: '', channel: '' })?.channel).toBe('beta');
	});

	it('ignores a build time it cannot read', () => {
		expect(describeCore({ version: '', commit: '', builtAt: 'yesterday', channel: 'nightly' })?.builtAt).toBeNull();
	});
});

describe('describeBuildLong', () => {
	it('lists everything known', () => {
		expect(describeBuildLong(build({ channel: 'nightly' }))).toBe(`nightly · ${SHA} · 2026-10-04T14:02:09Z`);
		expect(describeBuildLong(build({ channel: 'beta', version: 'beta-26.5-4', builtAt: null, commit: null }))).toBe(
			'beta 26.5 #4'
		);
	});
});
