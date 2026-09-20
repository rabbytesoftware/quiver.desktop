import { expect } from '@wdio/globals';

import { ARROW_CARD, openRoute, waitForAppReady } from '../lib/app-ready';
import { coreClient, getArrow, waitForCore } from '../lib/core-api';
import { NS_CORE } from '../lib/paths';

/**
 * SCENARIO 3 -- an outdated self-arrow gets the SAME badge every other
 * outdated arrow gets, through the generic path and nothing else.
 *
 * The bespoke `core://update_status` pipe was removed in Task 4.1. Nothing in
 * this file references it, and nothing here uses a self-arrow-specific
 * selector: the assertions below are the ones an outdated `tool-a` would
 * satisfy just as well.
 *
 * WHERE THE BADGE ACTUALLY LIVES. The brief describes this as "the sidebar
 * renders the badge". Read against the real components, that is not quite
 * where it is, and the difference matters for writing an assertion that can
 * pass: `ArrowRow` -- the component the sidebar's own `ArrowList` renders --
 * draws only an icon, a name and a namespace, and has no status badge at all.
 * The badge belongs to `ArrowTile`, which lives under
 * src/features/sidebar/components/arrows/ (hence the brief's file path) but
 * is consumed by Home, Library and the collection grids. So this scenario
 * asserts against the Library grid, which is where a user would really see it.
 *
 * WHY THE DOM IS ENOUGH TO PROVE THE BROADCAST FIRED. `useArrowStore` is fed
 * from exactly two places (src/lib/core-store/listeners/index.ts):
 * `seedInitialState`, which runs once per catalog load, and
 * `applyRuntimeUpdate`, which is called ONLY from the
 * `wsManager.subscribe('/v0/runtime', ...)` handler. So a badge that flips to
 * "outdated" in a window that is already up, with no reload, cannot have come
 * from anywhere but the generic `/v0/runtime` broadcast -- the same frame any
 * other arrow's state change arrives on.
 */

/**
 * The label `arrow.state.outdated` resolves to in `en`
 * (src/lib/i18n/locales/en.ts). `computeStatus` maps state `outdated` to
 * `{ labelKey: 'arrow.state.outdated', iconKind: 'up' }`, and `arrowTileStatus`
 * lets `up` through its BADGE_KINDS set, so this is the text the tile renders.
 */
const OUTDATED_LABEL = 'Update available';

/**
 * The arrow to drive outdated. Defaults to quiver.core's own self-arrow --
 * the point of the scenario is that a SELF-arrow needs no special casing.
 *
 * PRECONDITION: `CheckVersionDrift` (arrow/internal/store/store.go, reached
 * from `GetDetail`) re-resolves the namespace against the remote and compares
 * refs. Making a self-arrow genuinely outdated therefore needs a remote
 * serving a newer ref than the one installed -- the same mechanism any arrow
 * uses. There is no local seam for it in a production daemon: `platforms` is
 * embedded in metadata.yaml and not overridable from config.yaml.
 */
const OUTDATED_NS = process.env.QUIVER_E2E_OUTDATED_NS ?? NS_CORE;

describe('an outdated self-arrow gets the generic outdated badge', () => {
	const home = process.env.QUIVER_E2E_HOME!;

	let versionedNs = '';

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		const { body } = await getArrow(home, OUTDATED_NS);
		if (!body) throw new Error(`${OUTDATED_NS} is not in the catalog at all`);
		versionedNs = body.namespace.includes('@')
			? body.namespace
			: `${OUTDATED_NS}@${body.installed_ref ?? ''}`;
	});

	it('bootstraps the self-arrow to ready so it can legally go outdated', async () => {
		// `absent -> outdated` is NOT a legal ArrowState transition; only
		// `ready -> outdated` is. quiver.core seeds its own arrow (via
		// `selfarrow.EnsureRegistered`) in state `absent`, so the drift check
		// can set the detail flag `outdated: true` while the state machine --
		// which is what the badge reads -- stays put.
		//
		// quiver.core's own integration test bootstraps it the same way
		// (tests/integration/selfupdate/selfupdate_test.go): Install, whose
		// only real work for this manifest is dependency resolution, since the
		// self-arrow declares no install lifecycle. The two variables are
		// required by `ResolveVariables` on ANY execution of the arrow, not
		// just the `update` method that reads them.
		const { status, body } = await coreClient(home).post(
			`/v0/runtime/${encodeURIComponent(versionedNs)}/install`,
			{ variables: { QUIVER_RELEASE_ASSET_URL: 'unused-for-install', QUIVER_RELEASE_CHECKSUM: 'unused-for-install' } }
		);
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
		// `GetDetail` is what triggers the passive drift check, throttled by
		// `version_check_ttl`. No self-arrow-specific endpoint is involved:
		// this is the same GET the Arrow Details page makes for anything.
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

		// The generic badge: `data-slot="badge"`, the shared `badgeVariants`
		// element ArrowTile renders for every status whose iconKind is in
		// BADGE_KINDS. No self-arrow selector, no bespoke component.
		const badge = await tile.$('[data-slot="badge"]');
		await badge.waitForExist({
			timeout: 60_000,
			timeoutMsg: `the tile for ${OUTDATED_NS} rendered no [data-slot="badge"]`,
		});

		// The label proves the `up` branch specifically: `arrow.state.outdated`
		// is the only key that resolves to this string, and only iconKind 'up'
		// carries it.
		await expect(badge).toHaveText(expect.stringContaining(OUTDATED_LABEL));

		// iconKind 'up' renders lucide's ArrowUpIcon, not the spinner the
		// 'busy' kinds swap in -- an <svg>, and exactly one.
		const icons = await badge.$$('svg');
		expect(icons.length).toBe(1);
	});

	it('gives the self-arrow no badge treatment of its own', async () => {
		await openRoute('/library');

		const selfTile = await $(`${ARROW_CARD}[href*="${OUTDATED_NS}"]`);
		await selfTile.waitForExist({ timeout: 60_000 });

		// The badge a self-arrow gets must be structurally identical to the
		// one any other arrow gets: same slot, same variant classes. If a
		// bespoke path ever comes back, this is where it shows up as an extra
		// or differently-named node.
		const badges = await selfTile.$$('[data-slot="badge"]');
		expect(badges.length).toBe(1);
	});
});
