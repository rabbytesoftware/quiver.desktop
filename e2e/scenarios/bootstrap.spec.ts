import { expect } from '@wdio/globals';
import fs from 'node:fs';

import { waitForAppReady, waitForSidebarArrow, sidebarArrowNames } from '../lib/app-ready';
import { getArrow, waitForCore } from '../lib/core-api';
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

	it('registers quiver.core as its own user-installed arrow', async () => {
		await waitForCore(home);

		const { status, body } = await getArrow(home, NS_CORE);

		expect(status).toBe(200);
		expect(body).not.toBeNull();
		expect(body!.namespace).toContain('quiver.core');
		expect(body!.user_installed).toBe(true);
	});

	it('registers quiver.desktop as its own user-installed arrow', async () => {
		await waitForCore(home);

		const { status, body } = await getArrow(home, NS_DESKTOP);

		expect(status).toBe(200);
		expect(body).not.toBeNull();
		expect(body!.namespace).toContain('quiver.desktop');
		expect(body!.user_installed).toBe(true);
	});

	it('shows both self-arrows in the sidebar the user actually sees', async () => {
		const names = await waitForSidebarArrow('Quiver Core');
		expect(names).toContain('Quiver Core');

		const all = await sidebarArrowNames();
		expect(all).toContain('Quiver');
	});
});
