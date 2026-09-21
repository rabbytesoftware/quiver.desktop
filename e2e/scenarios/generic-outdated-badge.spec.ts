import { expect } from '@wdio/globals';

import { ARROW_CARD, openRoute, waitForAppReady } from '../lib/app-ready';
import { coreClient, getArrow, waitForCore } from '../lib/core-api';
import { NS_CORE } from '../lib/paths';

// SCENARIO 3 -- an outdated self-arrow gets the SAME badge every other
// outdated arrow gets, through the generic path. Asserted against the
// Library grid: the sidebar's ArrowRow has no status badge, that lives on
// ArrowTile, which Home/Library/the collection grids all render.

// The label `arrow.state.outdated` resolves to in `en` (i18n/locales/en.ts).
const OUTDATED_LABEL = 'Update available';

// Defaults to quiver.core's own self-arrow, since the point of this scenario
// is that a self-arrow needs no special casing. Making it genuinely outdated
// needs a remote serving a newer ref than the one installed.
const OUTDATED_NS = process.env.QUIVER_E2E_OUTDATED_NS ?? NS_CORE;

describe('an outdated self-arrow gets the generic outdated badge', () => {
	const home = process.env.QUIVER_E2E_HOME!;

	let versionedNs = '';

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		const { body } = await getArrow(home, OUTDATED_NS);
		if (!body) throw new Error(`${OUTDATED_NS} is not in the catalog at all`);
		versionedNs = body.namespace.includes('@') ? body.namespace : `${OUTDATED_NS}@${body.installed_ref ?? ''}`;
	});

	it('bootstraps the self-arrow to ready so it can legally go outdated', async () => {
		// absent -> outdated is not a legal ArrowState transition, only
		// ready -> outdated is, so it needs an Install first.
		const { status, body } = await coreClient(home).post(`/v0/runtime/${encodeURIComponent(versionedNs)}/install`, {
			variables: {
				QUIVER_RELEASE_ASSET_URL: 'unused-for-install',
				QUIVER_RELEASE_CHECKSUM: 'unused-for-install',
			},
		});
		if (status !== 202) {
			throw new Error(
				`install of ${versionedNs} answered ${status}, expected 202. Body: ${JSON.stringify(body)}`
			);
		}

		const deadline = Date.now() + 120_000;
		let state = '';
		let last: unknown = null;
		while (Date.now() < deadline) {
			const detail = await getArrow(home, versionedNs);
			state = detail.body?.state ?? '';
			last = detail.body?.last_return ?? null;
			if (state === 'ready' || state === 'outdated') break;
			await new Promise((r) => setTimeout(r, 500));
		}
		if (state !== 'ready' && state !== 'outdated') {
			throw new Error(`${versionedNs} stuck in "${state}" after install. last_return: ${JSON.stringify(last)}`);
		}
	});

	it('reports the self-arrow as outdated through the ordinary arrow API', async () => {
		let state = '';
		const deadline = Date.now() + 120_000;
		while (Date.now() < deadline) {
			const { body } = await getArrow(home, versionedNs);
			state = body?.state ?? '';
			if (state === 'outdated') break;
			await new Promise((r) => setTimeout(r, 1_000));
		}

		if (state !== 'outdated') {
			throw new Error(
				`${OUTDATED_NS} never became "outdated" (last state: "${state}"). CheckVersionDrift ` +
					'compares the installed ref against the remote, so this needs a remote serving a ' +
					'NEWER ref than the one installed. Set QUIVER_E2E_OUTDATED_NS to an arrow whose ' +
					'host really offers one.'
			);
		}
		expect(state).toBe('outdated');
	});

	it('renders the same badge on the tile that any outdated arrow gets', async () => {
		await openRoute('/library');

		const tile = await $(`${ARROW_CARD}[href*="${OUTDATED_NS}"]`);
		await tile.waitForExist({
			timeout: 60_000,
			timeoutMsg: `no ${ARROW_CARD} tile for ${OUTDATED_NS} in the library grid`,
		});

		const badge = await tile.$('[data-slot="badge"]');
		await badge.waitForExist({
			timeout: 60_000,
			timeoutMsg: `the tile for ${OUTDATED_NS} rendered no [data-slot="badge"]`,
		});

		await expect(badge).toHaveText(expect.stringContaining(OUTDATED_LABEL));

		const icons = await badge.$$('svg');
		expect(icons.length).toBe(1);
	});

	it('gives the self-arrow no badge treatment of its own', async () => {
		await openRoute('/library');

		const selfTile = await $(`${ARROW_CARD}[href*="${OUTDATED_NS}"]`);
		await selfTile.waitForExist({ timeout: 60_000 });

		const badges = await selfTile.$$('[data-slot="badge"]');
		expect(badges.length).toBe(1);
	});
});
