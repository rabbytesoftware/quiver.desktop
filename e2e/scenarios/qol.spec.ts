import { browser, $, expect } from '@wdio/globals';

import { openRoute, waitForAppReady } from '../lib/app-ready';
import { checkNow, coreClient, getArrow, waitForArrow, waitForCore } from '../lib/core-api';
import { publishRepoTag, upstreamEnv } from '../lib/upstream';
import { button, openArrowPage, shot, waitForButton, waitForNoButton } from '../lib/ui';

// The pre-playtest QOL fixes, in the real built app against a real daemon:
//
//   1. a screen starts at the top rather than inheriting the last one's scroll
//   2. focusing the search field does not navigate; typing does
//   3. when an update finishes, the Update button goes away on its own, with
//      no reload and no navigation

const home = process.env.QUIVER_E2E_HOME!;
const upstream = upstreamEnv();
const UPDATABLE_NS = process.env.QUIVER_E2E_UPDATABLE_NS ?? '';
const UPDATABLE_REPO = 'rabbytesoftware/e2e-updatable';

/** The one scroller every route renders into (src/features/shell/components/app-shell.tsx). */
const SCROLLER = 'main > div.overflow-auto';
const SEARCH = 'input[aria-label="Search"]';

async function pathname(): Promise<string> {
	return browser.execute(() => window.location.pathname);
}

async function scrollTop(): Promise<number> {
	return browser.execute(
		(selector: string) => document.querySelector<HTMLElement>(selector)?.scrollTop ?? -1,
		SCROLLER
	);
}

describe('pre-playtest QOL fixes', () => {
	let identity = '';

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		if (!UPDATABLE_NS) throw new Error('QUIVER_E2E_UPDATABLE_NS must name the box fixture');
		const client = coreClient(home);
		expect([200, 201]).toContain((await client.post(`/v0/arrow/${encodeURIComponent(UPDATABLE_NS)}`)).status);
		identity = (await getArrow(home, UPDATABLE_NS)).body!.namespace;
		expect((await client.post(`/v0/runtime/${encodeURIComponent(identity)}/install`, {})).status).toBe(202);
		await waitForArrow(home, identity, (d) => d.state === 'ready', 'the updatable arrow installed');
	});

	describe('scroll is not carried between screens', () => {
		it('starts the next screen at the top', async () => {
			const OFFSET = 300;
			await openArrowPage(identity);
			await $(SCROLLER).waitForExist();

			// A tall node React does not own, appended to the scroller: while the
			// next route is still rendering a skeleton the page would otherwise be
			// short, and the browser clamps the old offset to 0 by itself -- which
			// would pass this check without the shell doing anything.
			await browser.execute((selector: string) => {
				const spacer = document.createElement('div');
				spacer.id = 'qol-spacer';
				spacer.style.height = '4000px';
				document.querySelector<HTMLElement>(selector)!.appendChild(spacer);
			}, SCROLLER);
			await browser.execute(
				(selector: string, top: number) => {
					document.querySelector<HTMLElement>(selector)!.scrollTop = top;
				},
				SCROLLER,
				OFFSET
			);
			await browser.waitUntil(async () => (await scrollTop()) === OFFSET, {
				timeout: 15_000,
				timeoutMsg: 'the scroller never scrolled, so there is nothing to carry over',
			});
			await shot('01-scrolled');

			await openRoute('/settings');
			await browser.waitUntil(async () => (await pathname()) === '/settings', { timeout: 15_000 });
			expect(await $('#qol-spacer').isExisting()).toBe(true);

			expect(await scrollTop()).toBe(0);
			await shot('02-next-screen-at-top');
		});
	});

	describe('the search field', () => {
		it('stays where it is when focused or clicked', async () => {
			await openRoute('/settings');
			await browser.waitUntil(async () => (await pathname()) === '/settings', { timeout: 15_000 });

			await $(SEARCH).click();
			await browser.waitUntil(
				async () => (await browser.execute(() => document.activeElement?.tagName)) === 'INPUT',
				{
					timeout: 10_000,
					timeoutMsg: 'the search field never took focus',
				}
			);

			// Navigation is asynchronous; the claim is that none starts, so give a
			// would-be redirect a real chance to land by asking the router to idle.
			await browser.execute(
				() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve)))
			);
			expect(await pathname()).toBe('/settings');
			await shot('03-search-focused');
		});

		it('navigates to the results once the user types', async () => {
			await $(SEARCH).setValue('redis');
			await browser.waitUntil(
				async () =>
					(await pathname()) === '/search' &&
					(await browser.execute(() => window.location.search)) === '?q=redis',
				{ timeout: 15_000, timeoutMsg: 'typing never navigated to /search?q=redis' }
			);
			await shot('04-search-typed');
		});
	});

	describe('an update finishing drops its button', () => {
		it('shows Update once a newer release is upstream', async () => {
			await openArrowPage(identity);
			expect(await button('Update').isExisting()).toBe(false);

			publishRepoTag(upstream, UPDATABLE_REPO, 'stable-1.1');
			await checkNow(home, identity);
			await waitForArrow(home, identity, (d) => d.state === 'outdated', 'the updatable arrow outdated');
			await waitForButton('Update', 60_000);
			await shot('05-update-available');
		});

		it('removes the button when the update completes, without a reload or navigation', async () => {
			const pathBefore = await pathname();
			await browser.execute(() => {
				(window as unknown as { __qolMarker: string }).__qolMarker = 'same-document';
			});

			await button('Update').click();
			await waitForArrow(
				home,
				identity,
				(d) => d.resolved_ref === 'stable-1.1' && !d.available && d.state === 'ready' && d.active_run == null,
				'the update committed in core'
			);

			await waitForNoButton('Update', 60_000);
			expect(await pathname()).toBe(pathBefore);
			expect(await browser.execute(() => (window as unknown as { __qolMarker?: string }).__qolMarker)).toBe(
				'same-document'
			);
			await shot('06-update-done');
		});
	});
});
