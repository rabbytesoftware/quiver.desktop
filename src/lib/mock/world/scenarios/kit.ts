import type { StepProgress } from '@/domain/arrow';

import { MOCK_HOST_PLATFORM, type MockArrow, type MockMethod, type MockTarget, type MockVariable } from '../types';
import { iconFor } from './media';

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

export const INSTALL_STEPS = [
	'Resolve manifest',
	'Fetch archive',
	'Verify checksum',
	'Unpack to workdir',
	'Write runtime config',
];

export const START_STEPS = ['Bind ports', 'Start process', 'Await readiness'];

export function stepsAllDone(titles: string[]): StepProgress[] {
	return titles.map((title, index) => ({ index, title, status: 'completed', type: 'exec' }));
}

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

export function arrow(seed: ArrowSeed): MockArrow {
	return {
		ref: 'v1.0.0',
		version: '1.0.0',
		description: '',
		license: 'MIT',
		tags: [],
		// What a published arrow actually carries: its own icon, and no banner.
		// A fixture that wants one passes `banner: bannerFor(ns)` explicitly.
		icon: iconFor(seed.namespace),
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
