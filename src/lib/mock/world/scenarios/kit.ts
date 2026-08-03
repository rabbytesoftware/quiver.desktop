// Shared parts for building scenario datasets. A scenario is a list of arrows
// and collections; this is what keeps the list readable instead of a wall of
// twenty-field object literals.

import type { StepProgress } from '@/domain/arrow';

import { MOCK_HOST_PLATFORM, type MockArrow, type MockMethod, type MockTarget, type MockVariable } from '../types';

/**
 * A fixed instant, so nothing in a scenario reads the wall clock.
 *
 * Every scenario must build byte-identically on every run — otherwise
 * screenshots drift between machines and `buildWorld` cannot be asserted
 * deep-equal to itself, which is the only cheap guard against a scenario
 * quietly acquiring a random source.
 */
export const EPOCH = '2026-07-14T09:20:00Z';

export const HOST_PLATFORM = MOCK_HOST_PLATFORM;
export const OTHER_PLATFORM = 'linux/amd64';

export function method(
	name: string,
	description: string,
	availableIn: Array<'ready' | 'running'>,
	steps: string[]
): MockMethod {
	return { name, description, available_in: availableIn, steps };
}

export function target(platform: string, methods: MockMethod[]): MockTarget {
	return {
		platform,
		methods: Object.fromEntries(methods.map((m) => [m.name, m])),
	};
}

/** The five steps a plain install walks. Named, because the timeline shows them. */
export const INSTALL_STEPS = [
	'Resolve manifest',
	'Fetch archive',
	'Verify checksum',
	'Unpack to workdir',
	'Write runtime config',
];

export const START_STEPS = ['Bind ports', 'Start process', 'Await readiness'];

/** A completed run's step list, for an arrow whose last action succeeded. */
export function stepsAllDone(titles: string[]): StepProgress[] {
	return titles.map((title, index) => ({ index, title, status: 'completed', type: 'exec' }));
}

/** A run that died partway. Everything before `failedAt` completed; nothing after ran. */
export function stepsFailedAt(titles: string[], failedAt: number, error: string): StepProgress[] {
	return titles.map((title, index) => ({
		index,
		title,
		status: index < failedAt ? 'completed' : index === failedAt ? 'failed' : 'pending',
		type: 'exec',
		...(index === failedAt ? { error } : {}),
	}));
}

export function variable(
	name: string,
	description: string,
	type: MockVariable['type'],
	rest: Omit<MockVariable, 'name' | 'description' | 'type'> = {}
): MockVariable {
	return { name, description, type, ...rest };
}

type ArrowSeed = Partial<MockArrow> & Pick<MockArrow, 'namespace' | 'name' | 'state'>;

/**
 * Fills in everything a scenario did not bother to say.
 *
 * The defaults are the boring case on purpose: in the library, one ref, no
 * media, one target for the host platform with a `start`/`stop` pair. A
 * scenario entry then reads as its DIFFERENCES from that — which is the only
 * part worth reading, since the differences are what each entry exists to put
 * on screen.
 */
export function arrow(seed: ArrowSeed): MockArrow {
	return {
		ref: 'v1.0.0',
		version: '1.0.0',
		description: '',
		license: 'MIT',
		tags: [],
		icon: null,
		banner: null,
		maintainers: ['rabbyte'],
		url: `https://${seed.namespace}`,
		user_installed: true,
		installed_at: EPOCH,
		requirement: { cpu_cores: 2, memory_gb: 4, disk_gb: 10 },
		netbridge: [],
		variables: [],
		targets: [
			target(HOST_PLATFORM, [
				method('start', 'Start the service', ['ready'], START_STEPS),
				method('stop', 'Stop the service', ['running'], ['Signal process', 'Await exit']),
			]),
		],
		active_run: null,
		last_return: null,
		...seed,
	};
}
