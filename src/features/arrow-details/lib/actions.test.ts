import { describe, expect, it } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowDetail, ArrowLifecycle, ArrowState, ArrowTarget } from '@/domain/arrow';

import { computeActions } from './actions';

const PLATFORM = 'darwin/arm64';

const LIFECYCLE: ArrowLifecycle = {
	install: [runStep('Fetch archive')],
	update: [runStep('Fetch new version')],
	execute: [runStep('Start process')],
	stop: [signalStep('Signal process')],
	uninstall: [runStep('Remove workdir')],
};

const TARGET: ArrowTarget = {
	platform: PLATFORM,
	requirement: { cpu_cores: 1, memory_gb: 1, disk_gb: 1 },
	lifecycle: LIFECYCLE,
	methods: [],
};

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: 'github.com/rabbyte/minecraft@v1.21.4',
		name: 'Minecraft Server',
		description: '',
		license: 'MIT',
		url: '',
		tags: [],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [{ name: 'server-name', description: '', type: 'string' }],
		targets: [TARGET],
		state: 'ready',
		user_installed: true,
		installed_ref: 'v1.21.4',
		active_run: null,
		last_return: null,
		versions: [],
		readme: null,
		dependencies: [],
		dependents: [],
		...overrides,
	};
}

function kinds(state: ArrowState, opts: Partial<ArrowDetail> = {}) {
	return computeActions(detail({ state, ...opts }), PLATFORM).map((a) => a.kind);
}

describe('computeActions', () => {
	it('offers only Add to Library when the arrow is not in the library, regardless of state', () => {
		const actions = computeActions(detail({ user_installed: false, state: 'absent' }), PLATFORM);
		expect(actions).toHaveLength(1);
		expect(actions[0]).toMatchObject({ kind: 'addToLibrary', forceBusy: false, forceDisabled: false });
	});

	it('absent: Install (idle) + Remove from Library', () => {
		const actions = computeActions(detail({ state: 'absent' }), PLATFORM);
		expect(actions.map((a) => a.kind)).toEqual(['install', 'removeFromLibrary']);
		expect(actions[0]).toMatchObject({ forceBusy: false, forceDisabled: false, steps: LIFECYCLE.install });
	});

	it('installing: Install shows busy with its busy label, Remove from Library is disabled', () => {
		const actions = computeActions(detail({ state: 'installing' }), PLATFORM);
		expect(actions[0]).toMatchObject({ kind: 'install', forceBusy: true, busyLabelKey: 'arrow.action.installing' });
		expect(actions[1]).toMatchObject({ kind: 'removeFromLibrary', forceDisabled: true, forceBusy: false });
	});

	it('ready: Start (enabled) + Uninstall, when the target has an execute lifecycle', () => {
		expect(kinds('ready')).toEqual(['execute', 'uninstall']);
		const actions = computeActions(detail({ state: 'ready' }), PLATFORM);
		expect(actions[0]).toMatchObject({ forceDisabled: false, forceBusy: false, steps: LIFECYCLE.execute });
	});

	it('ready: omits Start entirely when the target has no execute lifecycle (a package with nothing to run)', () => {
		const noExecute = detail({
			state: 'ready',
			targets: [{ ...TARGET, lifecycle: { ...LIFECYCLE, execute: [] } }],
		});
		expect(computeActions(noExecute, PLATFORM).map((a) => a.kind)).toEqual(['uninstall']);
	});

	it('outdated: Update (enabled) + Start, hard-disabled unconditionally -- never gated by manifest data', () => {
		const actions = computeActions(detail({ state: 'outdated' }), PLATFORM);
		expect(actions[0]).toMatchObject({ kind: 'update', forceDisabled: false, forceBusy: false });
		expect(actions[1]).toMatchObject({ kind: 'execute', forceDisabled: true, forceBusy: false });
	});

	it('updating: Update shows busy, Start stays hard-disabled', () => {
		const actions = computeActions(detail({ state: 'updating' }), PLATFORM);
		expect(actions[0]).toMatchObject({ kind: 'update', forceBusy: true, busyLabelKey: 'arrow.action.updating' });
		expect(actions[1]).toMatchObject({ kind: 'execute', forceDisabled: true });
	});

	it('running: Stop + Restart, both enabled, Restart sequences stop then execute steps', () => {
		const actions = computeActions(detail({ state: 'running' }), PLATFORM);
		expect(actions.map((a) => a.kind)).toEqual(['stop', 'restart']);
		expect(actions[0]).toMatchObject({ forceBusy: false, forceDisabled: false, steps: LIFECYCLE.stop });
		expect(actions[1].steps).toEqual([...LIFECYCLE.stop, ...LIFECYCLE.execute]);
	});

	it('running: omits Restart when there is no execute lifecycle', () => {
		const noExecute = detail({
			state: 'running',
			targets: [{ ...TARGET, lifecycle: { ...LIFECYCLE, execute: [] } }],
		});
		expect(computeActions(noExecute, PLATFORM).map((a) => a.kind)).toEqual(['stop']);
	});

	it('stopping: Stop shows busy, Restart is disabled, and there is no third escalation action', () => {
		const actions = computeActions(detail({ state: 'stopping' }), PLATFORM);
		expect(actions).toHaveLength(2);
		expect(actions[0]).toMatchObject({ kind: 'stop', forceBusy: true, busyLabelKey: 'arrow.action.stopping' });
		expect(actions[1]).toMatchObject({ kind: 'restart', forceDisabled: true });
	});

	it('draining: same shape as stopping, with its own busy label', () => {
		const actions = computeActions(detail({ state: 'draining' }), PLATFORM);
		expect(actions[0]).toMatchObject({ kind: 'stop', forceBusy: true, busyLabelKey: 'arrow.action.draining' });
	});

	it('detached: a single, plain, enabled Stop -- not a distinct "force stop" action', () => {
		const actions = computeActions(detail({ state: 'detached' }), PLATFORM);
		expect(actions).toHaveLength(1);
		expect(actions[0]).toMatchObject({
			kind: 'stop',
			forceBusy: false,
			forceDisabled: false,
			steps: LIFECYCLE.stop,
		});
	});

	it('uninstalling: Start hard-disabled, Uninstall shows busy', () => {
		const actions = computeActions(detail({ state: 'uninstalling' }), PLATFORM);
		expect(actions[0]).toMatchObject({ kind: 'execute', forceDisabled: true });
		expect(actions[1]).toMatchObject({
			kind: 'uninstall',
			forceBusy: true,
			busyLabelKey: 'arrow.action.uninstalling',
		});
	});

	it('removed: only Reinstall, using the same install steps as a fresh install', () => {
		const actions = computeActions(detail({ state: 'removed' }), PLATFORM);
		expect(actions).toEqual([expect.objectContaining({ kind: 'reinstall', steps: LIFECYCLE.install })]);
	});

	it('picks the target matching the current platform, not just the first one', () => {
		const other: ArrowTarget = { ...TARGET, platform: 'linux/amd64', lifecycle: { ...LIFECYCLE, execute: [] } };
		const mine: ArrowTarget = { ...TARGET, platform: PLATFORM };
		const actions = computeActions(detail({ state: 'ready', targets: [other, mine] }), PLATFORM);
		expect(actions.map((a) => a.kind)).toEqual(['execute', 'uninstall']);
	});

	it('falls back to the first target when none matches the current platform', () => {
		const onlyOther: ArrowTarget = { ...TARGET, platform: 'linux/amd64' };
		const actions = computeActions(detail({ state: 'ready', targets: [onlyOther] }), 'windows/amd64');
		expect(actions.map((a) => a.kind)).toEqual(['execute', 'uninstall']);
	});

	it('every action that consumes variables lists every declared variable name (core has no per-action scoping)', () => {
		const withTwoVars = detail({
			state: 'ready',
			variables: [
				{ name: 'server-name', description: '', type: 'string' },
				{ name: 'difficulty', description: '', type: 'select' },
			],
		});
		const [start] = computeActions(withTwoVars, PLATFORM);
		expect(start.usesVariables).toEqual(['server-name', 'difficulty']);
	});

	it('an action with no core-provided step list (Add to Library, Remove from Library) previews an empty step list, never a fabricated one', () => {
		const actions = computeActions(detail({ state: 'absent' }), PLATFORM);
		const removeFromLibrary = actions.find((a) => a.kind === 'removeFromLibrary')!;
		expect(removeFromLibrary.steps).toEqual([]);
	});

	describe('an arrow with no targets at all (e.g. just added, manifest not yet resolved)', () => {
		it('every step list falls back to empty rather than throwing', () => {
			const noTargets = detail({ state: 'absent', targets: [] });
			expect(computeActions(noTargets, PLATFORM)[0].steps).toEqual([]);
		});

		it('treats it as having no execute lifecycle, so Start/Restart are omitted', () => {
			expect(kinds('ready', { targets: [] })).toEqual(['uninstall']);
			expect(kinds('running', { targets: [] })).toEqual(['stop']);
		});

		it('still returns a plain, enabled Stop for detached and running', () => {
			const runningActions = computeActions(detail({ state: 'running', targets: [] }), PLATFORM);
			expect(runningActions[0]).toMatchObject({
				kind: 'stop',
				steps: [],
				forceBusy: false,
				forceDisabled: false,
			});

			const detachedActions = computeActions(detail({ state: 'detached', targets: [] }), PLATFORM);
			expect(detachedActions[0]).toMatchObject({
				kind: 'stop',
				steps: [],
				forceBusy: false,
				forceDisabled: false,
			});
		});

		it('an unrecognized state (defensive against a wire value TypeScript cannot see) yields no actions rather than throwing', () => {
			expect(computeActions(detail({ state: 'nonsense' as unknown as ArrowState }), PLATFORM)).toEqual([]);
		});

		it('busy/uninstalling/removed states still resolve their kinds with empty step previews', () => {
			expect(kinds('installing', { targets: [] })).toEqual(['install', 'removeFromLibrary']);
			expect(kinds('outdated', { targets: [] })).toEqual(['update']);
			expect(kinds('updating', { targets: [] })).toEqual(['update']);
			expect(kinds('stopping', { targets: [] })).toEqual(['stop']);
			expect(kinds('draining', { targets: [] })).toEqual(['stop']);
			expect(kinds('uninstalling', { targets: [] })).toEqual(['uninstall']);
			expect(kinds('removed', { targets: [] })).toEqual(['reinstall']);
		});
	});
});
