import { describe, expect, it } from 'vitest';

import { runStep, signalStep } from '@/__mocks__/arrow-steps';
import type { ArrowDetail, ArrowTarget } from '@/domain/arrow';
import { QUIVER_DESKTOP_NAMESPACE } from '@/domain/release';

import { arrowMenuItems } from './arrow-menu';

const PLATFORM = 'darwin/arm64';

const TARGET: ArrowTarget = {
	platform: PLATFORM,
	requirement: { cpu_cores: 1, memory_gb: 1, disk_gb: 1 },
	lifecycle: {
		install: [runStep('Fetch')],
		update: [runStep('Fetch new')],
		execute: [runStep('Start')],
		stop: [signalStep('Stop')],
		uninstall: [runStep('Remove')],
	},
	methods: [],
};

function detail(overrides: Partial<ArrowDetail> = {}): ArrowDetail {
	return {
		namespace: 'github.com/rabbyte/minecraft@v1',
		name: 'Minecraft',
		description: '',
		license: 'MIT',
		url: '',
		tags: [],
		media: { icon: null, banner: null },
		maintainers: [],
		credits: [],
		netbridge: [],
		variables: [],
		targets: [TARGET],
		state: 'ready',
		user_installed: true,
		selector: 'v1',
		selector_kind: 'pin',
		resolved_ref: 'v1',
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

describe('arrowMenuItems', () => {
	it('mirrors the details page: Open leads for an openable ready arrow', () => {
		const items = arrowMenuItems(detail({ openable: true }), PLATFORM);
		expect(items.map((i) => i.kind)).toEqual(['open', 'execute', 'uninstall']);
		expect(items.every((i) => !i.viaDetails)).toBe(true);
	});

	it('routes an action that needs variables through the details page', () => {
		const items = arrowMenuItems(
			detail({ variables: [{ name: 'server-name', description: '', type: 'string' }] }),
			PLATFORM
		);
		expect(items.find((i) => i.kind === 'execute')?.viaDetails).toBe(true);
		expect(items.find((i) => i.kind === 'uninstall')?.viaDetails).toBe(false);
	});

	it("routes Quiver's own install/update through the details page", () => {
		const items = arrowMenuItems(
			detail({
				namespace: `${QUIVER_DESKTOP_NAMESPACE}@stable`,
				available: { ref: 'v2', commit: 'abc' },
				outdated: true,
			}),
			PLATFORM
		);
		expect(items.find((i) => i.kind === 'update')?.viaDetails).toBe(true);
	});
});
