import { describe, it, expect } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';

import type { ArrowDetailDTO, ArrowListResponseItemDTO, ArrowManifestDTO, ChannelListDTO } from './arrow';
import {
	toArrowCatalogRecords,
	toArrowChannels,
	toArrowDependencies,
	toArrowDetail,
	toInitialRuntimeUpdates,
} from './arrow';

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
						{ ref: '1.0.0', resolved_ref: '1.0.0', state: 'ready', installed_at: '2026-05-09T21:26:59Z' },
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
					versions: [{ ref: '1', resolved_ref: '1', state: 'ready' }],
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
					versions: [{ ref: '1', resolved_ref: '1', state: 'ready' }],
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
						{ ref: '1', resolved_ref: '1', state: 'ready' },
						{ ref: '2', resolved_ref: '2', state: 'absent' },
					],
				},
			],
			'local'
		);
		expect(records.map((r) => r.namespace)).toEqual(['a@1', 'a@2']);
	});

	it('files each row under its identity selector and carries the resolved ref as its version', () => {
		const records = toArrowCatalogRecords(
			[
				{
					namespace: 'github.com/char2cs/crowbar',
					name: 'crowbar',
					description: '',
					tags: [],
					versions: [
						{ ref: 'stable', resolved_ref: 'v1.3.0', state: 'ready' },
						{ ref: 'v1.*', resolved_ref: 'v1.3.0', state: 'absent' },
						{ ref: 'nightly', resolved_ref: '', state: 'absent' },
					],
				},
			],
			'local'
		);
		expect(records.map((r) => [r.namespace, r.version])).toEqual([
			['github.com/char2cs/crowbar@stable', 'v1.3.0'],
			['github.com/char2cs/crowbar@v1.*', 'v1.3.0'],
			['github.com/char2cs/crowbar@nightly', ''],
		]);
	});

	it('reads a version row from an older core with no resolved_ref as unresolved', () => {
		const [record] = toArrowCatalogRecords(
			[
				{
					namespace: 'a',
					name: 'a',
					description: '',
					tags: [],
					versions: [{ ref: '1', state: 'ready' } as ArrowListResponseItemDTO['versions'][number]],
				},
			],
			'local'
		);
		expect(record.version).toBe('');
	});
});

describe('origin on the arrow list', () => {
	const item = (extra: Partial<ArrowListResponseItemDTO> = {}): ArrowListResponseItemDTO => ({
		namespace: 'a',
		name: 'a',
		description: '',
		tags: [],
		versions: [
			{ ref: '1', resolved_ref: '1', state: 'ready' },
			{ ref: '2', resolved_ref: '2', state: 'ready' },
		],
		...extra,
	});

	it('carries an inferred arrow and its confidence onto every version record', () => {
		const records = toArrowCatalogRecords([item({ origin: 'inferred', confidence: 'medium' })], 'local');
		expect(records.map((r) => [r.origin, r.confidence])).toEqual([
			['inferred', 'medium'],
			['inferred', 'medium'],
		]);
	});

	it('reads a declared arrow as declared with no confidence', () => {
		const [record] = toArrowCatalogRecords([item({ origin: 'declared' })], 'local');
		expect(record.origin).toBe('declared');
		expect(record.confidence).toBeNull();
	});

	it('reads an older daemon that sends neither field as declared', () => {
		const [record] = toArrowCatalogRecords([item()], 'local');
		expect(record.origin).toBe('declared');
		expect(record.confidence).toBeNull();
	});

	it('drops a confidence it does not know', () => {
		const [record] = toArrowCatalogRecords([item({ origin: 'inferred', confidence: 'certain' })], 'local');
		expect(record.origin).toBe('inferred');
		expect(record.confidence).toBeNull();
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
				versions: [{ ref: '1', resolved_ref: '1', state: 'running' }],
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
					{ ref: '1', resolved_ref: '1', state: 'ready' },
					{ ref: '2', resolved_ref: '2', state: 'absent' },
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
				versions: [{ ref: '1', resolved_ref: '1', state: 'running' }],
			},
		]);
		expect(updates[0].active_run).toBeNull();
		expect(updates[0].last_return).toBeNull();
	});
});

const DETAIL: ArrowDetailDTO = {
	namespace: 'github.com/rabbyte/minecraft@stable',
	name: 'Minecraft Server',
	description: 'A server.',
	license: 'MIT',
	state: 'ready',
	tags: ['game'],
	installed_at: '2026-05-09T21:26:59Z',
	user_installed: true,
	selector_kind: 'channel',
	resolved_ref: 'v1.21.4',
	installed_commit: '3f2a9c1d',
	outdated: false,
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
		metadata: {
			name: 'Minecraft Server',
			description: 'A server.',
			license: 'MIT',
			url: 'https://github.com/rabbyte/minecraft',
			maintainers: [{ name: 'rabbyte', url: 'https://rabbyte.dev' }],
			credits: [{ name: 'Mojang' }],
			media: { icon: 'icon.png', banner: 'banner.png' },
		},
		variables: null,
		netbridge: [{ name: 'game', protocol: 'tcp', default: 25565, required: true }],
		targets: {},
	},
};

describe('toArrowDependencies', () => {
	it('casts type through and maps each entry', () => {
		expect(
			toArrowDependencies([
				{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' },
				{ namespace: 'github.com/rabbyte/postgres@v17.2', type: 'service' },
			])
		).toEqual([
			{ namespace: 'github.com/rabbyte/nats@v2.10.0', type: 'tool' },
			{ namespace: 'github.com/rabbyte/postgres@v17.2', type: 'service' },
		]);
	});

	it('returns an empty list for an empty input', () => {
		expect(toArrowDependencies([])).toEqual([]);
	});

	it('defaults a null input to an empty list, same as the wire can send', () => {
		expect(toArrowDependencies(null as unknown as [])).toEqual([]);
	});
});

describe('toArrowChannels', () => {
	it('maps an ordered channel through with its count and members intact', () => {
		const dto: ChannelListDTO = {
			channels: [
				{
					name: 'stable',
					kind: 'ordered',
					latest: 'stable-1.2.0',
					count: 3,
					members: ['stable-1.2.0', 'stable-1.1.0', 'stable-1.0.0'],
				},
			],
		};
		expect(toArrowChannels(dto)).toEqual([
			{
				name: 'stable',
				kind: 'ordered',
				latest: 'stable-1.2.0',
				count: 3,
				members: ['stable-1.2.0', 'stable-1.1.0', 'stable-1.0.0'],
			},
		]);
	});

	it('maps a pointer channel through with count/members left undefined, never fabricated', () => {
		const dto: ChannelListDTO = { channels: [{ name: 'nightly', kind: 'pointer', latest: 'nightly-latest' }] };
		const [result] = toArrowChannels(dto);
		expect(result).toEqual({ name: 'nightly', kind: 'pointer', latest: 'nightly-latest' });
		expect(result.count).toBeUndefined();
		expect(result.members).toBeUndefined();
	});

	it('returns an empty list for an empty channels array', () => {
		expect(toArrowChannels({ channels: [] })).toEqual([]);
	});
});

describe('toArrowDetail origin', () => {
	it('reads an inferred arrow and the confidence of its inference', () => {
		const result = toArrowDetail(
			{ ...DETAIL, origin: 'inferred', inference: { generator: 'fletcher/1', confidence: 'low' } },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.origin).toBe('inferred');
		expect(result.confidence).toBe('low');
	});

	it('reads an inferred arrow whose inference block is missing as inferred with no confidence', () => {
		const result = toArrowDetail({ ...DETAIL, origin: 'inferred' }, MANIFEST, [], null, [], []);
		expect(result.origin).toBe('inferred');
		expect(result.confidence).toBeNull();
	});

	it('reads a declared arrow, and an older daemon that sends no origin, as declared', () => {
		expect(toArrowDetail({ ...DETAIL, origin: 'declared' }, MANIFEST, [], null, [], []).origin).toBe('declared');
		const older = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(older.origin).toBe('declared');
		expect(older.confidence).toBeNull();
	});
});

describe('toArrowDetail', () => {
	it('carries the inference warnings, and none for a hand-written arrow', () => {
		const inferred = toArrowDetail(
			{
				...DETAIL,
				origin: 'inferred',
				inference: { generator: 'fletcher/1', confidence: 'medium', warnings: ['assumed_arch'] },
			},
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(inferred.warnings).toEqual(['assumed_arch']);
		expect(toArrowDetail(DETAIL, MANIFEST, [], null, [], []).warnings).toEqual([]);
	});

	it('keeps the identity quiver.core sends, selector included', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.namespace).toBe('github.com/rabbyte/minecraft@stable');
		expect(result.selector).toBe('stable');
	});

	it('reads the row state: selector kind, resolved ref and installed commit', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.selector_kind).toBe('channel');
		expect(result.resolved_ref).toBe('v1.21.4');
		expect(result.installed_commit).toBe('3f2a9c1d');
		expect(result.available).toBeNull();
		expect(result.outdated).toBe(false);
	});

	it('reads what is ahead from available, and outdated from it', () => {
		const result = toArrowDetail(
			{ ...DETAIL, available: { ref: 'v1.22.0', commit: 'abc1234' }, outdated: true },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.available).toEqual({ ref: 'v1.22.0', commit: 'abc1234' });
		expect(result.outdated).toBe(true);
	});

	it('derives outdated from available when a payload leaves it out', () => {
		const partial = { ...DETAIL, available: { ref: 'v1.22.0', commit: 'abc1234' } } as Partial<ArrowDetailDTO>;
		delete partial.outdated;
		expect(toArrowDetail(partial as ArrowDetailDTO, MANIFEST, [], null, [], []).outdated).toBe(true);
	});

	it('reads a payload from an older core, with none of the row-state fields, as a pin with nothing resolved', () => {
		const legacy = {
			namespace: 'github.com/rabbyte/minecraft@v1.21.4',
			name: 'Minecraft Server',
			description: 'A server.',
			state: 'ready',
			tags: null,
			user_installed: true,
		} as unknown as ArrowDetailDTO;
		const result = toArrowDetail(legacy, { ...MANIFEST, manifest: null }, [], null, [], []);
		expect(result.selector_kind).toBe('pin');
		expect(result.resolved_ref).toBe('');
		expect(result.installed_commit).toBe('');
		expect(result.available).toBeNull();
		expect(result.outdated).toBe(false);
		expect(result.license).toBe('');
		expect(result.url).toBe('');
		expect(result.media).toEqual({ icon: null, banner: null });
	});

	it('reads a selector kind it does not know as a pin', () => {
		const result = toArrowDetail(
			{ ...DETAIL, selector_kind: 'wildcard' as ArrowDetailDTO['selector_kind'] },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.selector_kind).toBe('pin');
	});

	it('ignores an available with no ref', () => {
		const result = toArrowDetail({ ...DETAIL, available: { ref: '', commit: '' } }, MANIFEST, [], null, [], []);
		expect(result.available).toBeNull();
	});

	it('reads an available with no commit as an empty commit', () => {
		const result = toArrowDetail(
			{ ...DETAIL, available: { ref: 'v1.22.0' } as ArrowDetailDTO['available'] },
			MANIFEST,
			[],
			null,
			[],
			[]
		);
		expect(result.available).toEqual({ ref: 'v1.22.0', commit: '' });
	});

	it('reads null targets as none', () => {
		const result = toArrowDetail(
			DETAIL,
			{ ...MANIFEST, targets: null as unknown as ArrowManifestDTO['targets'] },
			[],
			null,
			[],
			[]
		);
		expect(result.targets).toEqual([]);
	});

	it('falls back to the manifest license when the detail carries none', () => {
		const partial = { ...DETAIL } as Partial<ArrowDetailDTO>;
		delete partial.license;
		expect(toArrowDetail(partial as ArrowDetailDTO, MANIFEST, [], null, [], []).license).toBe('MIT');
	});

	it('sources url/maintainers/credits/media from the nested raw manifest, not the base detail call', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.url).toBe('https://github.com/rabbyte/minecraft');
		expect(result.maintainers).toEqual([{ name: 'rabbyte', email: undefined, url: 'https://rabbyte.dev' }]);
		expect(result.credits).toEqual([{ name: 'Mojang', email: undefined, url: undefined }]);
		expect(result.media).toEqual({ icon: 'icon.png', banner: 'banner.png' });
		expect(result.netbridge).toEqual(MANIFEST.manifest?.netbridge);
	});

	it('defaults media icon/banner to null rather than undefined when the manifest omits them', () => {
		const result = toArrowDetail(
			DETAIL,
			{
				...MANIFEST,
				manifest: { ...MANIFEST.manifest!, metadata: { ...MANIFEST.manifest!.metadata, media: {} } },
			},
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

	it('takes channels from the third argument, not either DTO', () => {
		const channels = [
			{ name: 'stable', kind: 'ordered' as const, latest: 'v1.21.4', count: 1, members: ['v1.21.4'] },
		];
		const result = toArrowDetail(DETAIL, MANIFEST, channels, null, [], []);
		expect(result.channels).toBe(channels);
	});

	it('carries the rest of the base detail fields straight through', () => {
		const result = toArrowDetail(DETAIL, MANIFEST, [], null, [], []);
		expect(result.name).toBe('Minecraft Server');
		expect(result.description).toBe('A server.');
		expect(result.license).toBe('MIT');
		expect(result.tags).toEqual(['game']);
		expect(result.state).toBe('ready');
		expect(result.user_installed).toBe(true);
		expect(result.installed_at).toBe('2026-05-09T21:26:59Z');
	});
});

/**
 * An arrow whose manifest declares no `tags:` arrives with `tags: null`, not
 * with an absent key and not with an empty array: Go marshals a nil slice as
 * JSON null and quiver.core passes the slice straight through. BOTH of
 * Quiver's own self-arrows are in exactly that state.
 *
 * This is not hypothetical and it was not cheap. Opening Quiver's own page in
 * the real app crashed the whole view with "null is not an object (evaluating
 * 'e.tags.length')" -- the hero reads `detail.tags.length` to decide whether
 * to render the tags row -- which meant the Update button on that page could
 * not be reached at all, by anyone, whatever it sent. Caught by clicking
 * through the running app in the E2E box; no unit test could have seen it,
 * because every fixture in this file had been written with a tags array.
 */
describe('an arrow with no tags at all', () => {
	const listItem = {
		namespace: 'github.com/rabbytesoftware/quiver.desktop',
		name: 'Quiver',
		description: 'The Quiver desktop application.',
		tags: null,
		versions: [{ ref: 'stable', resolved_ref: 'stable-1.0', state: 'ready' as const }],
	};

	it('comes back from the catalog mapper with an empty list, not null', () => {
		const records = toArrowCatalogRecords([listItem], 'local');
		expect(records[0].tags).toEqual([]);
	});

	it('comes back from the detail mapper with an empty list, not null', () => {
		const detail = toArrowDetail({ ...DETAIL, tags: null }, MANIFEST, [], null, [], []);
		expect(detail.tags).toEqual([]);
	});

	// Same nil-slice shape, same boundary. Neither self-manifest declares
	// `netbridge:`, so this one is not hypothetical either -- MetaPanel reads
	// `netbridge.length` exactly the way the hero read `tags.length`.
	it('defaults every other list the wire can send as null', () => {
		const detail = toArrowDetail(
			{ ...DETAIL, tags: null },
			{
				...MANIFEST,
				variables: null,
				manifest: {
					...MANIFEST.manifest!,
					metadata: { ...MANIFEST.manifest!.metadata, maintainers: null, credits: null },
					netbridge: null,
				},
			},
			[],
			null,
			null as unknown as [],
			null as unknown as []
		);
		expect(detail.netbridge).toEqual([]);
		expect(detail.maintainers).toEqual([]);
		expect(detail.credits).toEqual([]);
		expect(detail.variables).toEqual([]);
		expect(detail.dependencies).toEqual([]);
		expect(detail.dependents).toEqual([]);
	});
});
