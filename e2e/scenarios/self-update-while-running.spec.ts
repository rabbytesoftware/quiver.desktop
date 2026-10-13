import { browser, expect } from '@wdio/globals';

import { waitForAppReady } from '../lib/app-ready';
import {
	checkNow,
	coreClient,
	daemonVersion,
	getArrow,
	processAlive,
	waitForArrow,
	waitForCore,
} from '../lib/core-api';
import { NS_CORE } from '../lib/paths';
import { publishCoreRelease, upstreamEnv } from '../lib/upstream';

// SCENARIO 2 -- a process quiver.core supervises is not killed by quiver.core's
// own self-update, driven through the real Desktop app and a real swap of the
// daemon process (quiver.core's own swap_test.go proves the same without the
// app).
//
// Runs in the E2E box (docker/e2e/scenarios/run-wdio.sh): the new release is
// published to its GitHub stand-in and the daemon resolves it itself.
const FIXTURE_NS = process.env.QUIVER_E2E_FIXTURE_NS ?? '';

describe('self-update while a supervised process is running', () => {
	const home = process.env.QUIVER_E2E_HOME!;
	const upstream = upstreamEnv();
	const [OLD, NEW] = upstream.coreTags;
	let selfNs = '';
	let fixtureNs = '';
	let supervisedPid = 0;

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		const { body } = await getArrow(home, NS_CORE);
		expect(body).not.toBeNull();
		selfNs = body!.namespace;
		expect(selfNs).toContain('@');
		await waitForArrow(home, selfNs, (d) => d.state === 'ready' || d.state === 'outdated', 'a settled quiver.core');
		expect(await daemonVersion(home)).toBe(OLD);
	});

	after(() => {
		try {
			process.kill(supervisedPid, 'SIGKILL');
		} catch {
			/* already gone */
		}
	});

	it('has a supervised arrow to protect', async () => {
		if (!FIXTURE_NS) {
			throw new Error(
				'QUIVER_E2E_FIXTURE_NS is unset. This scenario needs an arrow a REAL daemon can resolve and run ' +
					'a long-lived `execute` for; the E2E box publishes one (docker/e2e/fixtures/arrows/e2e-supervised).'
			);
		}

		const added = await coreClient(home).post(`/v0/arrow/${encodeURIComponent(FIXTURE_NS)}`);
		expect([200, 201]).toContain(added.status);
		fixtureNs = (await getArrow(home, FIXTURE_NS)).body!.namespace;
	});

	it('installs and starts the supervised arrow', async () => {
		const client = coreClient(home);

		const install = await client.post(`/v0/runtime/${encodeURIComponent(fixtureNs)}/install`, {});
		expect(install.status).toBe(202);
		await waitForArrow(home, fixtureNs, (d) => d.state === 'ready', 'the supervised arrow installed');

		const exec = await client.post(`/v0/runtime/${encodeURIComponent(fixtureNs)}/execute`, {});
		expect(exec.status).toBe(202);
		const running = await waitForArrow(
			home,
			fixtureNs,
			(d) => d.state === 'running' && (d.active_run?.pid ?? 0) > 0,
			'the supervised arrow running'
		);
		supervisedPid = running.active_run!.pid!;
	});

	it('keeps the supervised process alive across the daemon replacing itself', async () => {
		publishCoreRelease(upstream, NEW);
		expect((await checkNow(home, selfNs)).status).toBe(200);
		await waitForArrow(home, selfNs, (d) => d.available?.ref === NEW, `${NEW} found ahead`);

		const { status } = await coreClient(home).post(`/v0/runtime/${encodeURIComponent(selfNs)}/update`, {});
		expect(status).toBe(202);

		await browser.waitUntil(async () => (await daemonVersion(home)) === NEW, {
			timeout: 180_000,
			interval: 500,
			timeoutMsg: `the daemon never reported ${NEW}`,
		});
		await waitForCore(home, 120_000);
		await waitForArrow(
			home,
			selfNs,
			(d) => d.resolved_ref === NEW && !d.available,
			'the new build settled',
			120_000
		);

		// RecordDetached clears Execution, so there's no active_run.pid
		// left in the read model once Detached, so check the OS process.
		expect(processAlive(supervisedPid)).toBe(true);
		const after = await getArrow(home, fixtureNs);
		expect(after.status).toBe(200);
		expect(after.body!.state).toBe('detached');
	});

	it('leaves the Desktop window still usable afterwards', async () => {
		await waitForAppReady();
		await expect($('[data-slot="sidebar"]')).toBeExisting();
	});
});
