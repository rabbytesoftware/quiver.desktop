import { browser, expect } from '@wdio/globals';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

import { waitForAppReady } from '../lib/app-ready';
import { coreClient, daemonVersion, getArrow, processAlive, waitForArrow, waitForCore } from '../lib/core-api';
import { NS_CORE, quiverHome, selfInstalledCore, socketPath } from '../lib/paths';
import { appPids, closeAppWindow, daemonPids, quiverPids, waitUntilGone } from '../lib/processes';
import { builtAsset, publishCoreRelease, upstreamEnv, upstreamHits } from '../lib/upstream';
import {
	button,
	openArrowPage,
	openEngineSettings,
	pageText,
	settingDescription,
	settingValue,
	shot,
	waitForButton,
	waitForNoButton,
	waitForSettingValue,
} from '../lib/ui';

// quiver.core updates itself through the app: the Update button posts the
// ordinary `update` method, core downloads the release, verifies it against
// the release's checksums.txt and swaps the daemon process in place. Driven
// through the real release-built app and the real daemon it spawned; the
// releases come from the box's GitHub stand-in (docker/e2e/fixtures/upstream.py)
// over HTTPS by the real hostnames.
//
// One home for the whole file, so the phases walk one installation through
// five releases:
//
//   V1 -> V2  Update from the arrow page
//   V2 -> V3  Update from Settings, Engine
//   V3 -> V4  V4 lists a wrong checksum: refused, nothing changes
//   V3 -> V5  V5 swaps in but never becomes healthy: rolled back
//   then      quitting the app stops the daemon that is running by then

const home = process.env.QUIVER_E2E_HOME!;
const upstream = upstreamEnv();
const [V1, V2, V3, V4, V5] = upstream.coreTags;
const FIXTURE_NS = process.env.QUIVER_E2E_FIXTURE_NS ?? '';

const UP_TO_DATE = 'Up to date';
const CHECK = 'Check for updates';

function sha256(file: string): string {
	return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

async function waitFor(what: string, check: () => Promise<boolean>, timeout = 120_000): Promise<void> {
	await browser.waitUntil(check, { timeout, interval: 500, timeoutMsg: `timed out waiting for ${what}` });
}

async function waitForDaemonVersion(version: string, timeout = 180_000): Promise<void> {
	let seen: string | null = null;
	try {
		await browser.waitUntil(
			async () => {
				seen = await daemonVersion(home);
				return seen === version;
			},
			{ interval: 500, timeout }
		);
	} catch {
		throw new Error(`the daemon never reported ${version}; last answer ${JSON.stringify(seen)}`);
	}
}

function selfLog(): string {
	try {
		return fs.readFileSync(path.join(quiverHome(home), 'logs', 'self-update.log'), 'utf8');
	} catch {
		return '';
	}
}

function leftovers(): string[] {
	return fs.readdirSync(path.dirname(selfInstalledCore(home))).filter((name) => name.startsWith('quiver.old-'));
}

describe('update quiver.core from the app', () => {
	let coreIdentity = '';
	let supervisedIdentity = '';
	let supervisedPid = 0;
	let appBefore: number[] = [];

	async function check(): Promise<void> {
		await button(CHECK).click();
	}

	/** One daemon, on `version`, with its binary at the self path and nothing left beside it. */
	async function expectSwappedTo(tag: string): Promise<void> {
		await waitForDaemonVersion(tag);
		await waitForCore(home);
		await waitFor(
			`${tag} settled in the catalog`,
			async () => {
				const detail = (await getArrow(home, coreIdentity)).body;
				return detail?.resolved_ref === tag && !detail.available && detail.state === 'ready';
			},
			120_000
		);
		expect(daemonPids()).toHaveLength(1);
		expect(sha256(selfInstalledCore(home))).toBe(sha256(builtAsset(upstream, tag)));
		await waitFor('the previous binary to be removed', async () => leftovers().length === 0, 30_000);
		expect(fs.existsSync(socketPath(home))).toBe(true);
		expect(appPids()).toEqual(appBefore);
		expect(processAlive(supervisedPid)).toBe(true);
	}

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		const detail = (await getArrow(home, NS_CORE)).body;
		expect(detail).not.toBeNull();
		coreIdentity = detail!.namespace;
		await waitForArrow(
			home,
			coreIdentity,
			(d) => d.state === 'ready' || d.state === 'outdated',
			'a settled quiver.core'
		);
		expect(await daemonVersion(home)).toBe(V1);

		if (!FIXTURE_NS) throw new Error('QUIVER_E2E_FIXTURE_NS must name the box fixture');
		const client = coreClient(home);
		expect([200, 201]).toContain((await client.post(`/v0/arrow/${encodeURIComponent(FIXTURE_NS)}`)).status);
		supervisedIdentity = (await getArrow(home, FIXTURE_NS)).body!.namespace;
		expect((await client.post(`/v0/runtime/${encodeURIComponent(supervisedIdentity)}/install`, {})).status).toBe(
			202
		);
		await waitForArrow(home, supervisedIdentity, (d) => d.state === 'ready', 'the supervised arrow installed');
		expect((await client.post(`/v0/runtime/${encodeURIComponent(supervisedIdentity)}/execute`, {})).status).toBe(
			202
		);
		const running = await waitForArrow(
			home,
			supervisedIdentity,
			(d) => d.state === 'running' && (d.active_run?.pid ?? 0) > 0,
			'the supervised arrow running'
		);
		supervisedPid = running.active_run!.pid!;
		appBefore = appPids();
	});

	after(() => {
		try {
			process.kill(supervisedPid, 'SIGKILL');
		} catch {
			/* already gone */
		}
	});

	describe(`${V1} to ${V2}: Update from the arrow page`, () => {
		let daemonBefore: number[] = [];

		it('shows the installed and available versions in Settings, Engine', async () => {
			await openEngineSettings();
			await waitForSettingValue('Installed version', V1);
			expect(await settingValue('Available version')).toBe(UP_TO_DATE);
			await expect(button(CHECK)).toBeEnabled();
			expect(await button('Update').isExisting()).toBe(false);
			await shot('01-settings-up-to-date');
		});

		it('finds the newer build when asked to check', async () => {
			publishCoreRelease(upstream, V2);

			await check();
			await waitForSettingValue('Available version', V2);
			await waitForButton('Update');
			await shot('02-settings-update-available');
		});

		it('downloads, verifies and swaps, and the app stays up and reconnects', async () => {
			daemonBefore = daemonPids();
			expect(daemonBefore).toHaveLength(1);

			await openArrowPage(coreIdentity);
			await waitForButton('Update');
			await shot('03-arrow-page-update-available');
			await button('Update').click();

			await expectSwappedTo(V2);
			await waitForNoButton('Update', 120_000);
			await shot('04-arrow-page-after-swap');

			expect(daemonPids()).not.toEqual(daemonBefore);
			const served = `/rabbytesoftware/quiver.core/releases/download/${V2}`;
			expect(upstreamHits(`${served}/${upstream.coreAsset}`)).toBeGreaterThanOrEqual(1);
			expect(upstreamHits(`${served}/checksums.txt`)).toBeGreaterThanOrEqual(1);
		});

		it('shows the new installed version in Settings', async () => {
			await openEngineSettings();
			await waitForSettingValue('Installed version', V2);
			expect(await settingValue('Available version')).toBe(UP_TO_DATE);
			await shot('05-settings-after-swap');
		});

		it('leaves the supervised process alive and detached with the same pid', async () => {
			expect(processAlive(supervisedPid)).toBe(true);
			expect((await getArrow(home, supervisedIdentity)).body!.state).toBe('detached');
		});
	});

	describe(`${V2} to ${V3}: Update from Settings`, () => {
		it('downloads, verifies and swaps, and Settings shows the new version', async () => {
			publishCoreRelease(upstream, V3);
			const daemonBefore = daemonPids();

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V3);
			await button('Update').click();
			await shot('06-settings-updating');

			await expectSwappedTo(V3);
			await waitForSettingValue('Installed version', V3);
			await waitForSettingValue('Available version', UP_TO_DATE);
			await waitForNoButton('Update', 120_000);
			await shot('07-settings-after-swap');

			expect(daemonPids()).not.toEqual(daemonBefore);
			expect((await getArrow(home, supervisedIdentity)).body!.state).toBe('detached');
		});
	});

	describe(`failure paths from ${V3}`, () => {
		it('refuses a release whose checksum does not match, and changes nothing', async () => {
			publishCoreRelease(upstream, V4, { binaryOf: V3, wrongChecksum: true });
			const daemonBefore = daemonPids();
			const binaryBefore = sha256(selfInstalledCore(home));

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V4);
			await button('Update').click();

			await waitFor(
				'the update to be refused',
				async () => (await getArrow(home, coreIdentity)).body?.last_return?.outcome === 'failed',
				120_000
			);
			await waitFor(
				'the row to settle',
				async () => (await getArrow(home, coreIdentity)).body?.state === 'outdated'
			);

			expect(await daemonVersion(home)).toBe(V3);
			expect(daemonPids()).toEqual(daemonBefore);
			expect(sha256(selfInstalledCore(home))).toBe(binaryBefore);
			expect(leftovers()).toEqual([]);
			expect(processAlive(supervisedPid)).toBe(true);

			const failed = (await getArrow(home, coreIdentity)).body!.last_return as unknown as {
				steps: { status: string; error?: string }[];
			};
			const message = failed.steps.find((step) => step.status === 'failed')?.error ?? '';
			expect(message.toLowerCase()).toContain('checksum');

			await waitFor(
				'the message to show in Settings',
				async () => (await settingDescription(CHECK)).toLowerCase().includes('checksum'),
				30_000
			);
			await shot('08-settings-wrong-checksum');

			await openArrowPage(coreIdentity);
			await waitFor('the failed run to show on the arrow page', async () => (await pageText()).includes('Issue'));
			await shot('09-arrow-page-wrong-checksum');
		});

		it('rolls back a build that never becomes healthy, and ends Outdated on the old version', async () => {
			publishCoreRelease(upstream, V5, { binaryOf: V3, unhealthy: true });

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V5);
			await button('Update').click();

			await waitFor(
				'the swap to give up on the new build',
				async () => selfLog().includes('rolling back'),
				180_000
			);
			await waitForDaemonVersion(V3);
			await waitForCore(home);
			await waitFor(
				'the row to be Outdated again',
				async () => {
					const detail = (await getArrow(home, coreIdentity)).body;
					return detail?.state === 'outdated' && detail.available?.ref === V5;
				},
				120_000
			);

			expect(sha256(selfInstalledCore(home))).toBe(sha256(builtAsset(upstream, V3)));
			await waitFor('the aside binary to be gone', async () => leftovers().length === 0, 30_000);
			expect(daemonPids()).toHaveLength(1);
			expect(processAlive(supervisedPid)).toBe(true);
			expect(appPids()).toEqual(appBefore);

			await openEngineSettings();
			await waitForSettingValue('Installed version', V3);
			await waitForSettingValue('Available version', V5);
			await waitForButton('Update');
			await shot('10-settings-after-rollback');
		});
	});

	describe('quitting the app', () => {
		it('stops the daemon that is running now, and leaves no quiver process behind', async () => {
			const current = daemonPids();
			expect(current).toHaveLength(1);

			// Closed the way a person closes it, through the window manager.
			closeAppWindow();
			await waitUntilGone(appPids, 'the desktop app');
			await waitUntilGone(daemonPids, `the daemon (pid ${current[0]}) the app started`);

			expect(await daemonVersion(home)).toBeNull();
			expect(processAlive(current[0])).toBe(false);
			await waitUntilGone(quiverPids, 'any quiver process');
		});
	});
});
