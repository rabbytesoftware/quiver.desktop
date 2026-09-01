import type { ArrowStepDefinition, Overridable, SignalKind, StepProgress } from '@/domain/arrow';

import { MOCK_HOST_PLATFORM, type MockArrow, type MockMethod, type MockTarget, type MockVariable } from '../types';
import { iconFor } from './media';

export const EPOCH = '2026-07-14T09:20:00Z';

export const HOST_PLATFORM = MOCK_HOST_PLATFORM;
export const OTHER_PLATFORM = 'linux/amd64';

// Builders for the three step types the mock actually uses (nothing declares
// a `dependencies` step today) -- real, representative content per type
// (a real-looking command, a real-looking url) so the "inspect this step"
// modal has something genuine to show, not just a title repeated twice.
export function runStep(
	title: string,
	command: Overridable<string>,
	rest: { elevated?: Overridable<boolean>; timeout?: Overridable<string> } = {}
): ArrowStepDefinition {
	return { type: 'run', title, command, elevated: rest.elevated ?? false, timeout: rest.timeout ?? '30s' };
}

export function fetchStep(
	title: string,
	url: Overridable<string>,
	rest: { to?: Overridable<string>; checksum?: Overridable<string>; timeout?: Overridable<string> } = {}
): ArrowStepDefinition {
	return {
		type: 'fetch',
		title,
		url,
		to: rest.to ?? '{{workdir}}/download',
		checksum: rest.checksum ?? '',
		timeout: rest.timeout ?? '60s',
	};
}

export function signalStep(
	title: string,
	signal: Overridable<SignalKind>,
	rest: { timeout?: Overridable<string> } = {}
): ArrowStepDefinition {
	return { type: 'signal', title, signal, timeout: rest.timeout ?? '10s' };
}

export function method(
	name: string,
	description: string,
	availableIn: Array<'ready' | 'running'>,
	steps: ArrowStepDefinition[]
): MockMethod {
	return { name, description, available_in: availableIn, steps };
}

export function target(platform: string, methods: MockMethod[]): MockTarget {
	return {
		platform,
		methods: Object.fromEntries(methods.map((m) => [m.name, m])),
	};
}

export const INSTALL_STEPS: ArrowStepDefinition[] = [
	fetchStep('Resolve manifest', 'https://index.quiver.dev/v0/resolve/{{namespace}}', {
		to: '{{workdir}}/manifest.json',
		timeout: '15s',
	}),
	fetchStep(
		'Fetch archive',
		'https://github.com/{{namespace}}/releases/download/{{ref}}/{{name}}-{{os}}-{{arch}}.tar.gz',
		{
			to: '{{workdir}}/archive.tar.gz',
			timeout: '120s',
		}
	),
	runStep('Verify checksum', 'sha256sum -c archive.tar.gz.sha256', { timeout: '10s' }),
	runStep('Unpack to workdir', 'tar -xzf archive.tar.gz -C {{workdir}}', { timeout: '30s' }),
	runStep('Write runtime config', 'quiver-init --workdir {{workdir}} --vars {{variables}}', { timeout: '5s' }),
];

export const START_STEPS: ArrowStepDefinition[] = [
	runStep('Bind ports', '{{workdir}}/bin/{{name}} --bind {{ports}}', { timeout: '10s' }),
	runStep('Start process', '{{workdir}}/bin/{{name}} --config {{workdir}}/config.yaml', {
		elevated: false,
		timeout: '30s',
	}),
	runStep('Await readiness', 'curl --retry 10 --retry-delay 2 -sf http://localhost:{{port}}/healthz', {
		timeout: '30s',
	}),
];

export const UPDATE_STEPS: ArrowStepDefinition[] = [
	fetchStep(
		'Fetch new version',
		'https://github.com/{{namespace}}/releases/download/{{ref}}/{{name}}-{{os}}-{{arch}}.tar.gz',
		{
			to: '{{workdir}}/update.tar.gz',
			timeout: '120s',
		}
	),
	runStep('Verify checksum', 'sha256sum -c update.tar.gz.sha256', { timeout: '10s' }),
	signalStep('Stop previous version', 'graceful', { timeout: '15s' }),
	runStep('Migrate config', 'quiver-migrate --workdir {{workdir}} --to {{ref}}', { timeout: '10s' }),
];

export const STOP_STEPS: ArrowStepDefinition[] = [
	signalStep('Signal process', 'graceful', { timeout: '10s' }),
	runStep('Await exit', 'while kill -0 {{pid}} 2>/dev/null; do sleep 1; done', { timeout: '30s' }),
];

export const UNINSTALL_STEPS: ArrowStepDefinition[] = [
	signalStep('Stop process', 'kill', { timeout: '5s' }),
	runStep('Remove workdir', 'rm -rf {{workdir}}', { timeout: '10s' }),
	runStep('Prune runtime config', 'rm -rf {{config_dir}}', { timeout: '5s' }),
];

export function stepsAllDone(steps: ArrowStepDefinition[]): StepProgress[] {
	return steps.map((step, index) => ({ index, title: step.title, status: 'completed', type: 'exec' }));
}

export function stepsFailedAt(steps: ArrowStepDefinition[], failedAt: number, error: string): StepProgress[] {
	return steps.map((step, index) => ({
		index,
		title: step.title,
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
		credits: [],
		url: `https://${seed.namespace}`,
		user_installed: true,
		installed_at: EPOCH,
		requirement: { cpu_cores: 2, memory_gb: 4, disk_gb: 10 },
		netbridge: [],
		variables: [],
		targets: [
			target(HOST_PLATFORM, [
				method('start', 'Start the service', ['ready'], START_STEPS),
				method('stop', 'Stop the service', ['running'], STOP_STEPS),
			]),
		],
		active_run: null,
		last_return: null,
		...seed,
	};
}
