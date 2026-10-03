import { browser, $ } from '@wdio/globals';
import fs from 'node:fs';
import path from 'node:path';

import { openRoute, waitForAppReady } from './app-ready';

/** A button by its exact visible label, which is what a person finds it by. */
export function button(label: string): ChainablePromiseElement {
	return $(`//button[normalize-space(.)="${label}"]`);
}

export async function buttonExists(label: string): Promise<boolean> {
	return button(label).isExisting();
}

export async function waitForButton(label: string, timeout = 60_000): Promise<void> {
	await button(label).waitForExist({ timeout, timeoutMsg: `no button labelled "${label}" appeared within ${timeout}ms` });
}

export async function waitForNoButton(label: string, timeout = 60_000): Promise<void> {
	await button(label).waitForExist({
		reverse: true,
		timeout,
		timeoutMsg: `a button labelled "${label}" was still on screen after ${timeout}ms`,
	});
}

/** The arrow page of a catalog identity, reached the way a link in the app reaches it. */
export async function openArrowPage(identity: string): Promise<void> {
	await openRoute(`/arrow/${identity}`);
}

export async function openEngineSettings(): Promise<void> {
	await openRoute('/settings?tab=engine');
	await $('//span[normalize-space(.)="Installed version"]').waitForExist({
		timeout: 60_000,
		timeoutMsg: 'Settings, Engine never rendered its Installed version row',
	});
}

const settingRow = (label: string) => `//span[normalize-space(.)="${label}"]/ancestor::div[@role="presentation"][1]`;

/** The control text of one Settings row, such as the version it shows. */
export async function settingValue(label: string): Promise<string> {
	const control = await $(`${settingRow(label)}//*[@data-slot="setting-control"]`);
	return ((await control.getAttribute('textContent')) ?? '').trim();
}

/** The sentence under a Settings row's label. */
export async function settingDescription(label: string): Promise<string> {
	const row = await $(settingRow(label));
	return ((await row.getAttribute('textContent')) ?? '').trim();
}

export async function waitForSettingValue(label: string, expected: string, timeout = 60_000): Promise<void> {
	let seen = '';
	try {
		await browser.waitUntil(
			async () => {
				seen = await settingValue(label);
				return seen === expected;
			},
			{ timeout, interval: 500 }
		);
	} catch {
		throw new Error(`Settings row "${label}" never read "${expected}"; last read "${seen}"`);
	}
}

/**
 * Saves what the window shows to the run's results directory, so a green run
 * leaves the screens the assertions were made against and a red one leaves the
 * screen it failed on.
 */
export async function shot(name: string): Promise<void> {
	const dir = process.env.QUIVER_E2E_RESULTS;
	if (!dir) return;
	const folder = path.join(dir, process.env.QUIVER_E2E_SPEC ?? 'spec');
	fs.mkdirSync(folder, { recursive: true });
	try {
		await browser.saveScreenshot(path.join(folder, `${name}.png`));
	} catch {
		/* a session that is mid-restart has no window to capture */
	}
}

/** The visible text of the window's main content, for assertions on copy. */
export async function pageText(): Promise<string> {
	return ((await $('body').getAttribute('textContent')) ?? '').replace(/\s+/g, ' ').trim();
}

export { waitForAppReady };
