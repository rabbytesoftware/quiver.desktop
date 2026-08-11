import type { ArrowState } from '@/domain/arrow';

import { createRng, intBetween, pick } from '../rng';
import type { MockArrow, MockCollection, MockCollectionMember } from '../types';
import { arrow, INSTALL_STEPS, stepsAllDone } from './kit';

const SEED = 0x71ac3f;

const SUBJECTS = [
	'atlas',
	'beacon',
	'cinder',
	'drift',
	'ember',
	'fathom',
	'gossamer',
	'harbor',
	'ironwood',
	'juniper',
	'kestrel',
	'lantern',
	'meridian',
	'nimbus',
	'obsidian',
	'petrichor',
	'quarry',
	'rampart',
	'solstice',
	'tempest',
	'umbra',
	'verdant',
	'wayfarer',
	'xenon',
	'yarrow',
	'zephyr',
];

const KINDS = ['server', 'daemon', 'relay', 'store', 'gateway', 'runner', 'index', 'cache'];

const TAG_POOL = ['game', 'server', 'database', 'network', 'media', 'observability', 'storage', 'security', 'service'];

const STATE_POOL: ArrowState[] = [
	'ready',
	'ready',
	'ready',
	'ready',
	'ready',
	'ready',
	'running',
	'running',
	'running',
	'absent',
	'absent',
	'outdated',
	'outdated',
	'installing',
	'updating',
	'stopping',
	'draining',
	'detached',
	'uninstalling',
	'removed',
];

export function buildExtremeArrows(): MockArrow[] {
	const rng = createRng(SEED);
	const arrows: MockArrow[] = [];

	for (let i = 0; i < 200; i++) {
		const subject = SUBJECTS[i % SUBJECTS.length];
		const kind = KINDS[Math.floor(i / SUBJECTS.length) % KINDS.length];
		const slug = `${subject}-${kind}-${i}`;
		const state = pick(rng, STATE_POOL);
		const major = intBetween(rng, 0, 9);
		const minor = intBetween(rng, 0, 40);

		arrows.push(
			arrow({
				namespace: `github.com/quiver-demo/${slug}`,
				name: `${subject[0].toUpperCase()}${subject.slice(1)} ${kind[0].toUpperCase()}${kind.slice(1)}`,
				description: `Generated fixture ${i}. A ${kind} for ${subject} workloads.`,
				ref: `v${major}.${minor}.0`,
				version: `${major}.${minor}.0`,
				state,
				tags: [pick(rng, TAG_POOL), pick(rng, TAG_POOL)].filter((t, idx, all) => all.indexOf(t) === idx),
				user_installed: rng() > 0.2,
				requirement: {
					cpu_cores: intBetween(rng, 1, 16),
					memory_gb: intBetween(rng, 1, 64),
					disk_gb: intBetween(rng, 2, 500),
				},
				netbridge:
					rng() > 0.5
						? [
								{
									name: 'main',
									protocol: rng() > 0.5 ? 'tcp' : 'udp',
									default: intBetween(rng, 1024, 65000),
									required: true,
								},
							]
						: [],
				last_return:
					state === 'ready' || state === 'running'
						? { method: 'install', outcome: 'success', variables: {}, steps: stepsAllDone(INSTALL_STEPS) }
						: null,
			})
		);
	}

	return arrows;
}

export function buildExtremeCollections(arrows: MockArrow[]): MockCollection[] {
	const rng = createRng(SEED ^ 0x5eed);
	return Array.from({ length: 12 }, (_, i) => {
		const size = intBetween(rng, 4, 14);
		const members: MockCollectionMember[] = Array.from({ length: size }, () => {
			const m = pick(rng, arrows);
			return { namespace: `${m.namespace}@${m.ref}`, resolved: true };
		});
		members.push({
			namespace: `github.com/quiver-demo/missing-${i}@v1.0.0`,
			resolved: false,
			reason: 'host unreachable',
		});

		return {
			namespace: `github.com/quiver-demo/pack-${i}`,
			name: `Pack ${i + 1}`,
			description: `Generated collection ${i + 1}, ${size} members.`,
			maintainers: ['quiver-demo'],
			followed: i % 3 === 0,
			arrows: members.filter(
				(member, idx, all) => all.findIndex((x) => x.namespace === member.namespace) === idx
			),
		};
	});
}
