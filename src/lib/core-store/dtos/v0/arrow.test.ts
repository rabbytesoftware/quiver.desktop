import { describe, it, expect } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';

import type { ArrowDetailDTO, ArrowManifestDTO } from './arrow';
import { toArrowCatalogRecords, toArrowDetail, toInitialRuntimeUpdates } from './arrow';

describe('toArrowCatalogRecords', () => {
	it('reads icon and banner from the nested media object', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'github.com/x/y',
					name: 'y',
					description: 'd',
					tags: ['t'],
					media: { icon: 'i.png', banner: 'b.png' },
					versions: [
						{ ref: '1.0.0', version: '1.0.0', state: 'ready', installed_at: '2026-05-09T21:26:59Z' },
					],
				},
			],
			'local'
		);
		expect(records[0].icon).toBe('i.png');
		expect(records[0].banner).toBe('b.png');
	});

	it('tolerates an absent media object', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [{ ref: '1', version: '1', state: 'ready' }],
				},
			],
			'local'
		);
		expect(records[0].icon).toBeNull();
	});

	it('stamps every record with its connection', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [{ ref: '1', version: '1', state: 'ready' }],
				},
			],
			'remote-7'
		);
		expect(records[0].connectionId).toBe('remote-7');
	});

	it('produces one record per installed version', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [
						{ ref: '1', version: '1', state: 'ready' },
						{ ref: '2', version: '2', state: 'absent' },
					],
				},
			],
			'local'
		);
		expect(records.map((r) => r.namespace)).toEqual(['a@1', 'a@2']);
	});
});

describe('toInitialRuntimeUpdates', () => {
	it('carries versions[].state through as the initial state', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		expect(updates).toEqual([{ namespace: 'a@1', state: 'running', active_run: null, last_return: null }]);
	});

	it('produces one update per installed version, matching the catalog namespace scheme', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [
					{ ref: '1', version: '1', state: 'ready' },
					{ ref: '2', version: '2', state: 'absent' },
				],
			},
		]);
		expect(updates.map((u) => u.namespace)).toEqual(['a@1', 'a@2']);
	});

	it('always nulls active_run and last_return, since the list endpoint never carries them', () => {
		const updates = toInitialRuntimeUpdates([
			{
				namespace: 'a',
				name: 'a',
				description: '',
				tags: [],
				versions: [{ ref: '1', version: '1', state: 'running' }],
			},
		]);
		expect(updates[0].active_run).toBeNull();
		expect(updates[0].last_return).toBeNull();
	});
});

describe('toArrowDetail', () => {
	const DETAIL: ArrowDetailDTO = {
		namespace: 'github.com/rabbyte/minecraft',
		name: 'Minecraft Server',
		version: '1.21.4',
		description: 'A server.',
		license: 'MIT',
		state: 'ready',
		tags: ['game'],
		installed_ref: 'v1.21.4',
		installed_at: '2026-05-09T21:26:59Z',
		user_installed: true,
		active_run: null,
		last_return: null,
	};

	const MANIFEST: ArrowManifestDTO = {
		namespace: 'github.com/rabbyte/minecraft',
		name: 'Minecraft Server',
		description: 'A server.',
		tags: ['game'],
		variables: [{ name: 'server-name', description: 'Shown in the list.', type: 'string', default: 'My Server' }],
		targets: {
			'darwin/arm64': {
				requirements: { cpu_cores: 2, memory_gb: 4, disk_gb: 10 },
				lifecycle: {
					install: [runStep('Fetch archive')],
					update: [runStep('Fetch new version')],
					execute: [runStep('Start process')],
					stop: [signalStep('Signal process')],
					uninstall: [runStep('Remove workdir')],
				},
				methods: {
					backup: { name: 'backup', description: 'Snapshot the world.', available_in: ['ready'], steps: [] },
				},
			},
		},
		manifest: {
			url: 'https://github.com/rabbyte/minecraft',
			maintainers: [{ name: 'rabbyte', url: 'https://rabbyte.dev' }],
			credits: [{ name: 'Mojang' }],
			media: { icon: 'icon.png', banner: 'banner.png' },
			netbridge: [{ name: 'game', protocol: 'tcp', default: 25565, required: true }],
		},
	};

	it('combines the bare namespace with installed_ref, since every downstream call needs the full identifier', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.namespace).toBe('github.com/rabbyte/minecraft@v1.21.4');
	});

	it('sources url/maintainers/credits/media from the nested raw manifest, not the base detail call', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.url).toBe('https://github.com/rabbyte/minecraft');
		expect(result.maintainers).toEqual([{ name: 'rabbyte', email: undefined, url: 'https://rabbyte.dev' }]);
		expect(result.credits).toEqual([{ name: 'Mojang', email: undefined, url: undefined }]);
		expect(result.media).toEqual({ icon: 'icon.png', banner: 'banner.png' });
		expect(result.netbridge).toEqual(MANIFEST.manifest.netbridge);
	});

	it('defaults media icon/banner to null rather than undefined when the manifest omits them', () => {
		const result = toArrowDetail(
			DETAIL,
			{ ...MANIFEST, manifest: { ...MANIFEST.manifest, media: {} } },
			[],
			null,
			[],
			[]
		);
		expect(result.media).toEqual({ icon: null, banner: null });
	});

	it('maps each target, keyed by platform, with its own requirement/lifecycle/methods', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.targets).toHaveLength(1);
		const [target] = result.targets;
		expect(target.platform).toBe('darwin/arm64');
		expect(target.requirement).toEqual({ cpu_cores: 2, memory_gb: 4, disk_gb: 10 });
		expect(target.lifecycle.execute).toEqual([runStep('Start process')]);
		expect(target.methods).toEqual([
			{ name: 'backup', description: 'Snapshot the world.', available_in: ['ready'], steps: [] },
		]);
	});

	it('passes variables through from the manifest DTO', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.variables).toEqual(MANIFEST.variables);
	});

	it('defaults active_run/last_return to null when the base detail omits them', () => {
		const result = toArrowDetail(
			{ ...DETAIL, active_run: undefined, last_return: undefined },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.active_run).toBeNull();
		expect(result.last_return).toBeNull();
	});

	it('preserves a non-null active_run/last_return exactly', () => {
		const activeRun = { method: 'execute', variables: {}, steps: [] };
		const lastReturn = { method: 'install', outcome: 'success' as const, variables: {}, steps: [] };
		const result = toArrowDetail(
			{ ...DETAIL, active_run: activeRun, last_return: lastReturn },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.active_run).toEqual(activeRun);
		expect(result.last_return).toEqual(lastReturn);
	});

	it('passes a null readme through, since it comes from the fourth argument, not either DTO', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.readme).toBeNull();
	});

	it('passes a non-null readme through exactly', () => {
		const readme = '## About\n\nA server.';
		const result = toArrowDetail(DETAIL, MANIFEST, [], readme, [], []);
		expect(result.readme).toBe(readme);
	});

	it('takes versions from the third argument, not either DTO', () => {
		const versions = [{ ref: 'v1.21.4', version: '1.21.4', state: 'ready' as const }];
		const result = toArrowDetail(DETAIL, MANIFEST, versions, null, [], []);
		expect(result.versions).toBe(versions);
	});

	it('carries the rest of the base detail fields straight through', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.name).toBe('Minecraft Server');
		expect(result.description).toBe('A server.');
		expect(result.license).toBe('MIT');
		expect(result.tags).toEqual(['game']);
		expect(result.state).toBe('ready');
		expect(result.user_installed).toBe(true);
		expect(result.installed_ref).toBe('v1.21.4');
		expect(result.installed_at).toBe('2026-05-09T21:26:59Z');
	});
});
