import { expect } from '@wdio/globals';
import fs from 'node:fs';

import { waitForAppReady, waitForSidebarArrow, sidebarArrowNames } from '../lib/app-ready';
import { getArrow, getChannels, listLibrary, waitForCore } from '../lib/core-api';
import { NS_CORE, NS_DESKTOP, quiverHome, selfInstalledCore } from '../lib/paths';

// SCENARIO 1 -- bootstrap from a genuinely empty QUIVER_HOME (deleted by
// beforeSession in wdio.conf.ts), driving the real shipped artifact rather
// than the per-repo unit tests that only exercise pieces of this in isolation.
describe('bootstrap: a clean QUIVER_HOME comes up self-installed', () => {
	const home = process.env.QUIVER_E2E_HOME!;

	it('started from a home with no self-installed Core', () => {
		expect(process.env.QUIVER_E2E_CLEAN_HOME).toBe('true');
	});

	it('reaches a ready state in the real window', async () => {
		await waitForAppReady();
		await expect($('[data-slot="sidebar"]')).toBeExisting();
	});

	it('leaves a quiver.core binary at <QUIVER_HOME>/self/quiver', async () => {
		const selfBinary = selfInstalledCore(home);

		await browser.waitUntil(() => fs.existsSync(selfBinary), {
			timeout: 60_000,
			interval: 500,
			timeoutMsg: `quiver.core never self-installed to ${selfBinary} (QUIVER_HOME=${quiverHome(home)})`,
		});

		const stat = fs.statSync(selfBinary);
		expect(stat.isFile()).toBe(true);
		expect(stat.size).toBeGreaterThan(0);
		expect(stat.mode & 0o111).toBeGreaterThan(0);
	});

	it('registers quiver.core as its own user-installed arrow, under a selector', async () => {
		await waitForCore(home);

		const { status, body } = await getArrow(home, NS_CORE);

		expect(status).toBe(200);
		expect(body).not.toBeNull();
		// A catalog identity is always `namespace@selector`; a refless read
		// reaches the preferred row.
		expect(body!.namespace.startsWith(`${NS_CORE}@`)).toBe(true);
		expect(body!.user_installed).toBe(true);
		expect(body!.resolved_ref).toBeTruthy();
	});

	it('files quiver.desktop under a channel of its repository, never a pin of its version', async () => {
		await waitForCore(home);

		// This build is stamped with no release channel (e2e.yml builds it
		// with no VITE_QUIVER_BUILD_CHANNEL), so it announces the repository's
		// default channel: the first one /channels lists.
		const { status: channelsStatus, body: channels } = await getChannels(home, NS_DESKTOP);
		expect(channelsStatus).toBe(200);
		const names = (channels?.channels ?? []).map((c) => c.name);
		expect(names.length).toBeGreaterThan(0);
		const identity = `${NS_DESKTOP}@${process.env.QUIVER_E2E_BUILD_CHANNEL || names[0]}`;

		// The announce is fire-and-forget on connect, so the row lands a
		// moment after the window is ready.
		let detail = await getArrow(home, identity);
		await browser.waitUntil(
			async () => {
				detail = await getArrow(home, identity);
				return detail.status === 200;
			},
			{ timeout: 60_000, interval: 500, timeoutMsg: `quiver.desktop never announced itself as ${identity}` }
		);

		expect(detail.body!.namespace).toBe(identity);
		expect(detail.body!.selector_kind).toBe('channel');
		expect(detail.body!.user_installed).toBe(true);
		expect(detail.body!.resolved_ref).toBeTruthy();

		const { body: library } = await listLibrary(home);
		const selectors = (library ?? []).find((item) => item.namespace === NS_DESKTOP)?.versions.map((v) => v.ref);
		expect(selectors).toEqual([identity.slice(NS_DESKTOP.length + 1)]);
	});

	it('shows both self-arrows in the sidebar the user actually sees', async () => {
		const names = await waitForSidebarArrow('Quiver Core');
		expect(names).toContain('Quiver Core');

		// The sidebar lists the name in each arrow's manifest, and the
		// desktop's ARROW.md names it "Quiver Desktop".
		const all = await sidebarArrowNames();
		expect(all).toContain('Quiver Desktop');
	});
});
