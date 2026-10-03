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
		selector: 'v1.21.4',
		selector_kind: 'pin',
		resolved_ref: 'v1.21.4',
		installed_commit: '',
		available: null,
		outdated: false,
		active_run: null,
		last_return: null,
		channels: [],
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

	it('ready with something available: Update comes first, Start and Uninstall stay as they were', () => {
		const available = { ref: 'v1.22.0', commit: 'abc1234' };
		expect(kinds('ready', { available, outdated: true })).toEqual(['update', 'execute', 'uninstall']);
		const [update] = computeActions(detail({ state: 'ready', available, outdated: true }), PLATFORM);
		expect(update).toMatchObject({ forceDisabled: false, forceBusy: false, steps: LIFECYCLE.update });
	});

	it('ready with nothing available: no Update at all', () => {
		expect(kinds('ready', { available: null, outdated: false })).not.toContain('update');
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

/**
 * The configure form on Quiver's own tile must not ask for the two values the
 * app resolves for itself at click time. Asking would be asking a person to
 * paste a download URL and a sha256 that `hero.tsx` is about to overwrite.
 */
describe('computeActions on Quiver’s own row', () => {
	const SELF = 'github.com/rabbytesoftware/quiver.desktop@stable-1.0';
	const VARIABLES = [
		{ name: 'QUIVER_DESKTOP_APPIMAGE_PATH', description: '', type: 'string' as const, default: '/x' },
		{ name: 'QUIVER_RELEASE_ASSET_URL', description: '', type: 'string' as const },
		{ name: 'QUIVER_RELEASE_CHECKSUM', description: '', type: 'string' as const },
	];

	function usesOf(state: ArrowState, namespace: string, kind: string): string[] {
		const actions = computeActions(detail({ namespace, state, variables: VARIABLES }), PLATFORM);
		return actions.find((a) => a.kind === kind)?.usesVariables ?? [];
	}

	it('leaves the release variables out of install’s form', () => {
		expect(usesOf('absent', SELF, 'install')).toEqual(['QUIVER_DESKTOP_APPIMAGE_PATH']);
	});

	it('leaves them out of reinstall’s form too', () => {
		expect(usesOf('removed', SELF, 'reinstall')).toEqual(['QUIVER_DESKTOP_APPIMAGE_PATH']);
	});

	it('still asks for every declared variable on any other arrow', () => {
		expect(usesOf('absent', 'github.com/rabbyte/minecraft@v1', 'install')).toEqual([
			'QUIVER_DESKTOP_APPIMAGE_PATH',
			'QUIVER_RELEASE_ASSET_URL',
			'QUIVER_RELEASE_CHECKSUM',
		]);
	});
});

const STAGED = { version: 'stable-26.6', staged_at: '2026-10-03T10:00:00Z' };
const AHEAD = { ref: 'stable-26.6', commit: 'abc' };
const CORE = 'github.com/rabbytesoftware/quiver.core@stable';
const ORDINARY = 'github.com/rabbyte/minecraft@v1.21.4';

describe('computeActions has no special case for quiver.core', () => {
	it.each<[ArrowState, Partial<ArrowDetail>]>([
		['outdated', {}],
		['ready', { available: AHEAD, outdated: true }],
		['updating', {}],
		['ready', { pending_activation: STAGED }],
	])('gives quiver.core the same actions as any other arrow in %s', (state, extra) => {
		const core = computeActions(detail({ namespace: CORE, state, ...extra }), PLATFORM);
		const other = computeActions(detail({ namespace: ORDINARY, state, ...extra }), PLATFORM);
		expect(core).toEqual(other);
	});

	it('leaves Update enabled', () => {
		const update = computeActions(detail({ namespace: CORE, state: 'outdated' }), PLATFORM).find(
			(a) => a.kind === 'update'
		);
		expect(update?.forceDisabled).toBe(false);
	});
});

describe('computeActions with a staged activation', () => {
	it.each<[ArrowState, Partial<ArrowDetail>]>([
		['ready', { pending_activation: STAGED }],
		['ready', { pending_activation: STAGED, available: AHEAD, outdated: true }],
		['outdated', { pending_activation: STAGED, available: AHEAD, outdated: true }],
	])('swaps Update for Restart to apply in %s, for any arrow', (state, extra) => {
		for (const namespace of [CORE, ORDINARY]) {
			const actions = computeActions(detail({ namespace, state, ...extra }), PLATFORM);
			expect(actions.map((a) => a.kind)).not.toContain('update');
			expect(actions[0]).toMatchObject({
				kind: 'activate',
				labelKey: 'arrow.action.restartToApply',
				busyLabelKey: 'arrow.action.restartingToApply',
				variant: 'default',
				steps: [],
				usesVariables: [],
				forceBusy: false,
				forceDisabled: false,
			});
		}
	});

	it('keeps Start and Uninstall where they were in ready', () => {
		expect(kinds('ready', { pending_activation: STAGED })).toEqual(['activate', 'execute', 'uninstall']);
	});

	it('shows no Restart to apply when nothing is staged', () => {
		expect(kinds('ready', { pending_activation: null })).not.toContain('activate');
		expect(kinds('ready')).not.toContain('activate');
	});

	it('does not offer Restart to apply for an arrow that is not in the library', () => {
		expect(kinds('absent', { user_installed: false, pending_activation: STAGED })).toEqual(['addToLibrary']);
	});
});
