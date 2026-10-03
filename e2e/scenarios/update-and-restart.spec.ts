import { browser, expect } from '@wdio/globals';
import crypto from 'node:crypto';
import fs from 'node:fs';

import { waitForAppReady } from '../lib/app-ready';
import { quiverCli } from '../lib/cli';
import {
	coreClient,
	daemonVersion,
	getArrow,
	processAlive,
	waitForArrow,
	waitForCore,
	type ArrowDetailDTO,
} from '../lib/core-api';
import { NS_CORE, selfInstalledCore, socketPath } from '../lib/paths';
import { appPids, closeAppWindow, daemonPids, waitUntilGone } from '../lib/processes';
import { builtAsset, publishCoreRelease, publishRepoTag, upstreamEnv, upstreamHits } from '../lib/upstream';
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

// The update and Restart to apply flow of quiver.core, driven through the real
// release-built app and the real daemon it spawned: no mocks, no stubbed
// resolver. The daemon reaches its releases over HTTPS by the real hostnames,
// served by the box's GitHub stand-in (docker/e2e/fixtures/upstream.py), and
// replaces itself with builds that are really different programs.
//
// One home for the whole file, so the phases below walk one installation
// through four releases:
//
//   V1 -> V2  Update from the arrow page, Restart to apply from the arrow page
//   V2 -> V3  Update from Settings, then quit and relaunch without applying
//   V3 -> V4  Update and Restart to apply, both from Settings
//   V4 -> V5  V5 publishes no digest: the update must refuse and change nothing

const home = process.env.QUIVER_E2E_HOME!;
const upstream = upstreamEnv();
const [V1, V2, V3, V4, V5] = upstream.coreTags;
const FIXTURE_NS = process.env.QUIVER_E2E_FIXTURE_NS ?? '';
const REQUIRED_NS = process.env.QUIVER_E2E_REQUIRED_NS ?? '';

const RESTART_TO_APPLY = 'Restart to apply';
const RESTARTING = 'Restarting…';
const UPDATE_ROW = 'Update quiver.core';

function sha256(file: string): string {
	return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

/** What the daemon-side download of `tag` should have placed in <home>/self once that build runs. */
function sha256OfBuild(tag: string): string {
	return sha256(builtAsset(upstream, tag));
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

describe('update quiver.core and apply it with Restart to apply', () => {
	let coreIdentity = '';
	let supervisedIdentity = '';
	let supervisedPid = 0;

	async function core(): Promise<ArrowDetailDTO> {
		const { body } = await getArrow(home, coreIdentity);
		if (!body) throw new Error(`${coreIdentity} has no detail`);
		return body;
	}

	/** Nothing about the running system moved: same build, same daemon process, supervised arrow untouched. */
	async function expectUntouched(version: string, daemonBefore: number[]): Promise<void> {
		expect(await daemonVersion(home)).toBe(version);
		expect(daemonPids()).toEqual(daemonBefore);
		expect(processAlive(supervisedPid)).toBe(true);
	}

	async function stagedVersion(): Promise<string | undefined> {
		return (await core()).pending_activation?.version;
	}

	async function check(): Promise<void> {
		await button('Check for updates').click();
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

		if (!FIXTURE_NS || !REQUIRED_NS) {
			throw new Error('QUIVER_E2E_FIXTURE_NS and QUIVER_E2E_REQUIRED_NS must name the box fixtures');
		}
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
	});

	after(() => {
		try {
			process.kill(supervisedPid, 'SIGKILL');
		} catch {
			/* already gone */
		}
	});

	describe(`${V1} to ${V2}: Update and Restart to apply from the arrow page`, () => {
		let daemonBefore: number[] = [];
		let appBefore: number[] = [];

		it('shows the installed and available versions in Settings, Engine, with Check for updates', async () => {
			await openEngineSettings();
			await waitForSettingValue('Installed version', V1);
			expect(await settingValue('Available version')).toBe('None');
			expect(await settingDescription(UPDATE_ROW)).toContain('quiver.core is up to date.');
			await expect(button('Check for updates')).toBeEnabled();
			expect(await button('Update').isExisting()).toBe(false);
			await shot('01-settings-up-to-date');
		});

		it('finds the new release when asked to check', async () => {
			publishCoreRelease(upstream, V2);

			await check();
			await waitForSettingValue('Available version', V2);
			expect(await settingDescription(UPDATE_ROW)).toContain(`Version ${V2} is available.`);
			await waitForButton('Update');
			await shot('02-settings-update-available');
		});

		it('downloads and stages from the arrow page without restarting anything', async () => {
			daemonBefore = daemonPids();
			appBefore = appPids();
			expect(daemonBefore).toHaveLength(1);

			await openArrowPage(coreIdentity);
			await waitForButton('Update');
			await shot('03-arrow-page-update-available');
			await button('Update').click();
			await waitForButton(RESTART_TO_APPLY, 180_000);
			await shot('04-arrow-page-staged');

			expect(await stagedVersion()).toBe(V2);
			await expectUntouched(V1, daemonBefore);
			const supervised = (await getArrow(home, supervisedIdentity)).body!;
			expect(supervised.state).toBe('running');
			expect(supervised.active_run?.pid).toBe(supervisedPid);
			expect(
				upstreamHits(`/rabbytesoftware/quiver.core/releases/download/${V2}/${upstream.coreAsset}`)
			).toBeGreaterThanOrEqual(1);
		});

		it('offers Restart to apply in Settings too', async () => {
			await openEngineSettings();
			await waitForButton(RESTART_TO_APPLY);
			expect(await settingDescription(UPDATE_ROW)).toContain(`Version ${V2} is downloaded and checked.`);
			expect(await settingValue('Installed version')).toBe(V1);
			await shot('05-settings-staged');
		});

		it('shows the restarting state, reconnects, and is on the new version with nothing pending', async () => {
			await openArrowPage(coreIdentity);
			await waitForButton(RESTART_TO_APPLY);
			await button(RESTART_TO_APPLY).click();

			await waitFor('the restarting state', () => button(RESTARTING).isExisting(), 15_000);
			await shot('06-arrow-page-restarting');

			await waitForDaemonVersion(V2);
			await waitForNoButton(RESTARTING, 120_000);
			await waitForNoButton(RESTART_TO_APPLY);
			await shot('07-arrow-page-after-restart');

			expect(await stagedVersion()).toBeUndefined();
			expect((await core()).resolved_ref).toBe(V2);
			expect(appPids()).toEqual(appBefore);
			expect(daemonPids()).toEqual(daemonBefore);
			expect(fs.existsSync(socketPath(home))).toBe(true);

			await openEngineSettings();
			await waitForSettingValue('Installed version', V2);
			expect(await settingValue('Available version')).toBe('None');
			expect(await settingDescription(UPDATE_ROW)).toContain('quiver.core is up to date.');
			await shot('08-settings-after-restart');
		});

		it('keeps the supervised process alive and leaves it detached, as the handover promises', async () => {
			expect(processAlive(supervisedPid)).toBe(true);
			expect((await getArrow(home, supervisedIdentity)).body!.state).toBe('detached');
		});

		it('installs the new build as the self-installed binary', async () => {
			await waitFor(
				'the self-installed binary to be the new build',
				async () => {
					return (
						fs.existsSync(selfInstalledCore(home)) && sha256(selfInstalledCore(home)) === sha256OfBuild(V2)
					);
				},
				60_000
			);
		});
	});

	describe(`${V2} to ${V3}: Update from Settings, then quit and relaunch without applying`, () => {
		it('downloads and stages from Settings without restarting anything', async () => {
			publishCoreRelease(upstream, V3);
			const daemonBefore = daemonPids();

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V3);
			await button('Update').click();
			await waitForButton(RESTART_TO_APPLY, 180_000);
			await shot('09-settings-staged-v3');

			expect(await stagedVersion()).toBe(V3);
			await expectUntouched(V2, daemonBefore);
			expect(await settingValue('Installed version')).toBe(V2);

			await openArrowPage(coreIdentity);
			await waitForButton(RESTART_TO_APPLY);
		});

		it('quits the app, which takes the daemon it started with it', async () => {
			// Closed the way a person closes it, through the window manager. The
			// WebDriver "close window" command ends the app but, measured in this
			// box, leaves the daemon it started running; the close button stops it.
			closeAppWindow();
			await waitUntilGone(appPids, 'the desktop app');
			await waitUntilGone(daemonPids, 'the daemon the app started');
			expect(await daemonVersion(home)).toBeNull();
		});

		it('promotes the staged build at the next boot', async () => {
			await browser.reloadSession();
			await waitForAppReady();
			await waitForCore(home);
			await waitForDaemonVersion(V3);

			expect(await stagedVersion()).toBeUndefined();
			await waitFor(
				'the self-installed binary to be the staged build',
				async () => {
					return sha256(selfInstalledCore(home)) === sha256OfBuild(V3);
				},
				60_000
			);

			await openEngineSettings();
			await waitForSettingValue('Installed version', V3);
			expect(await button(RESTART_TO_APPLY).isExisting()).toBe(false);
			await shot('10-settings-after-boot-promotion');
		});
	});

	describe(`${V3} to ${V4}: Update and Restart to apply from Settings`, () => {
		it('stages, restarts on request, and ends on the new version with nothing pending', async () => {
			publishCoreRelease(upstream, V4);
			const daemonBefore = daemonPids();
			const appBefore = appPids();

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V4);
			await button('Update').click();
			await waitForButton(RESTART_TO_APPLY, 180_000);
			expect(await stagedVersion()).toBe(V4);
			await expectUntouched(V3, daemonBefore);

			await button(RESTART_TO_APPLY).click();
			await waitFor('the restarting state', () => button(RESTARTING).isExisting(), 15_000);
			expect(await settingDescription(UPDATE_ROW)).toContain('Restarting quiver.core.');
			await shot('11-settings-restarting');

			await waitForDaemonVersion(V4);
			await waitForNoButton(RESTARTING, 120_000);
			await waitForNoButton(RESTART_TO_APPLY);
			await waitForSettingValue('Installed version', V4);

			expect(await stagedVersion()).toBeUndefined();
			expect(appPids()).toEqual(appBefore);
			expect(daemonPids()).toEqual(daemonBefore);
			expect(processAlive(supervisedPid)).toBe(true);
			await shot('12-settings-after-restart');
		});
	});

	describe(`failure paths from ${V4}`, () => {
		it('refuses a release that publishes no digest, with the typed error, and changes nothing', async () => {
			publishCoreRelease(upstream, V5, { withoutDigest: true, binaryOf: V4 });
			const daemonBefore = daemonPids();
			const binaryBefore = sha256(selfInstalledCore(home));

			await openEngineSettings();
			await check();
			await waitForSettingValue('Available version', V5);
			await button('Update').click();
			await waitFor(
				'the error to show',
				async () => (await settingDescription(UPDATE_ROW)).includes('digest'),
				60_000
			);
			await shot('13-settings-no-digest');

			const shown = await settingDescription(UPDATE_ROW);
			expect(shown).toContain('This release publishes no checksum, so it cannot be verified.');
			expect(shown).toContain('release unresolved: unverifiable');
			expect(shown).not.toContain('Another update is underway');
			await waitForButton('Try again');

			await expectUntouched(V4, daemonBefore);
			expect(await stagedVersion()).toBeUndefined();
			expect(sha256(selfInstalledCore(home))).toBe(binaryBefore);
			expect(upstreamHits(`/rabbytesoftware/quiver.core/releases/download/${V5}/${upstream.coreAsset}`)).toBe(0);
		});

		it('shows the same typed error on the arrow page', async () => {
			await openArrowPage(coreIdentity);
			await waitForButton('Update');
			await button('Update').click();
			await waitFor('the error to show', async () => (await pageText()).includes('release unresolved'), 60_000);
			await shot('14-arrow-page-no-digest');

			const text = await pageText();
			expect(text).toContain('release unresolved: unverifiable');
			expect(text).toContain('release asset has no published digest');
			expect(text).not.toContain('Another update is underway');
			expect(await daemonVersion(home)).toBe(V4);
			expect(await stagedVersion()).toBeUndefined();
		});

		it('rejects a CLI update with an empty required variable, naming it, and starts nothing', async () => {
			const client = coreClient(home);
			const identity = `${REQUIRED_NS}@stable`;
			expect([200, 201]).toContain((await client.post(`/v0/arrow/${encodeURIComponent(identity)}`)).status);
			expect((await client.post(`/v0/runtime/${encodeURIComponent(identity)}/install`, {})).status).toBe(202);
			await waitForArrow(home, identity, (d) => d.state === 'ready', 'e2e-required installed');

			publishRepoTag(upstream, 'rabbytesoftware/e2e-required', 'stable-1.1');
			await client.patch(`/v0/arrow/${encodeURIComponent(identity)}`, {});
			await waitForArrow(home, identity, (d) => d.available?.ref === 'stable-1.1', 'e2e-required outdated');

			for (const args of [
				['update', identity, '--data', 'E2E_TOKEN='],
				['update', identity],
			]) {
				const result = quiverCli(home, args);
				expect(result.status).not.toBe(0);
				expect(`${result.stdout}\n${result.stderr}`).toContain('E2E_TOKEN');
			}

			const after = (await getArrow(home, identity)).body!;
			expect(after.resolved_ref).toBe('stable-1.0');
			expect(after.state).not.toBe('updating');
		});
	});
});
