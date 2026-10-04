import { browser, $, $$, expect } from '@wdio/globals';

import { waitForAppReady } from '../lib/app-ready';
import { coreClient, waitForCore } from '../lib/core-api';
import { shot } from '../lib/ui';

// SCENARIO -- the build indicator and the daemon console, driven through the
// real built app and the real quiver.core it spawned. No mock: what is asserted
// is the wire contract of docs/console-spec.md as the shipped daemon honours it,
// and the window a person actually sees.
//
// The daemon under test must have the console (`features` has `console.v1`):
// build the box with a quiver.core that includes it (QUIVER_CORE_DEV_PATH).

const INDICATOR = '[data-slot="build-indicator"] button';
const PANEL = '#console-panel';
const INPUT = '#console-input';

interface Versions {
	version?: string;
	commit?: string;
	built_at?: string;
	channel?: string;
	features?: string[];
}

async function openConsole(): Promise<void> {
	if ((await $(PANEL).getAttribute('data-open')) === 'true') return;
	await $(INDICATOR).click();
	await browser.waitUntil(async () => (await $(PANEL).getAttribute('data-open')) === 'true', {
		timeout: 10_000,
		timeoutMsg: 'the console never opened',
	});
}

/** Types a command the way a person does and waits for the prompt to take it. */
async function run(line: string): Promise<void> {
	const input = await $(INPUT);
	await input.waitForClickable({ timeout: 10_000 });
	await input.click();
	await input.setValue(line);
	await browser.keys('Enter');
	await browser.waitUntil(async () => (await input.getValue()) === '', {
		timeout: 10_000,
		timeoutMsg: `the console never took "${line}"`,
	});
}

async function consoleText(): Promise<string> {
	return ((await $('[data-slot="console-log"]').getAttribute('textContent')) ?? '').trim();
}

async function waitForConsoleText(pattern: RegExp, timeout = 60_000): Promise<void> {
	let seen = '';
	try {
		await browser.waitUntil(
			async () => {
				seen = await consoleText();
				return pattern.test(seen);
			},
			{ timeout, interval: 300 }
		);
	} catch {
		throw new Error(`the console never showed ${pattern}; it shows:\n${seen.slice(-1500)}`);
	}
}

describe('console: the build indicator and the daemon console', () => {
	const home = process.env.QUIVER_E2E_HOME!;

	it('starts with a daemon that advertises the console', async () => {
		await waitForAppReady();
		await waitForCore(home);

		const { status, body } = await coreClient(home).get<Versions>('/versions');
		expect(status).toBe(200);
		expect(body?.features).toContain('console.v1');
	});

	it('shows both builds in the rail, once the daemon has answered', async () => {
		await $(INDICATOR).waitForExist({ timeout: 30_000 });
		await browser.waitUntil(
			async () => {
				const text = ((await $(INDICATOR).getAttribute('textContent')) ?? '').replace(/\s+/g, ' ');
				return /core/.test(text) && /app/.test(text) && !text.includes('—');
			},
			{ timeout: 30_000, timeoutMsg: 'the indicator never showed both a core and an app build' }
		);
		await shot('console-indicator');
	});

	it('names the core build the way the daemon reports it', async () => {
		const { body } = await coreClient(home).get<Versions>('/versions');
		const title = (await $(INDICATOR).getAttribute('title')) ?? '';
		expect(title).toContain('quiver.core:');
		if (body?.commit) expect(title).toContain(body.commit);
	});

	it('keeps the console unreachable until it is opened', async () => {
		await expect($(PANEL)).toHaveAttribute('data-open', 'false');
		await expect($(PANEL)).toHaveAttribute('inert');
	});

	it('opens from the indicator and shows the daemon\'s own log, typed', async () => {
		await openConsole();
		await $('[data-slot="log-row"]').waitForExist({ timeout: 30_000, timeoutMsg: 'the daemon log never appeared' });

		const rows = await $$('[data-slot="log-row"]');
		const count = rows.length;
		expect(count).toBeGreaterThan(0);
		// Every row carries a level the styling keys off.
		for (let i = 0; i < Math.min(count, 20); i++) {
			expect(['debug', 'info', 'warn', 'error']).toContain(await rows[i].getAttribute('data-level'));
		}
		await shot('console-open');
	});

	it('expands a line to its raw JSON on click', async () => {
		const first = await $('[data-slot="log-row"] button');
		await first.click();
		await $('[data-slot="log-json"]').waitForExist({ timeout: 5_000 });
		expect(await $('[data-slot="log-json"]').getAttribute('textContent')).toContain('"msg"');
		await first.click();
	});

	it('runs a command on the daemon and prints what it said', async () => {
		await run('version');
		await waitForConsoleText(/quiver/i);
		await $('[data-slot="console-output"]').waitForExist({ timeout: 30_000 });
	});

	it('runs a read command to a clean exit', async () => {
		await run('list');
		await browser.pause(1500);
		// A clean exit says nothing extra; a failure ends in an alert line.
		expect((await $$('[data-slot="console-note"][data-tone="error"]')).length).toBe(0);
	});

	it('refuses what the daemon does not offer, in the daemon\'s own words', async () => {
		await run('daemon');
		await waitForConsoleText(/Refused \(403\)/);
		await run('add github.com/char2cs/crowbar');
		await waitForConsoleText(/not available in the console/);
		await shot('console-refused');
	});

	it('never waits on a confirmation: a destructive command without --yes refuses at once', async () => {
		await run('arrow remove github.com/example/not-there');
		await waitForConsoleText(/requires --yes/);
		expect((await $$('[data-slot="console-note"][data-tone="error"]')).length).toBeGreaterThan(0);
	});

	it('logs the commands it ran, as the daemon records them', async () => {
		await waitForConsoleText(/exec\s+device=local\s+line=version/, 30_000);
	});

	it('refuses, on the wire, what the console must never run', async () => {
		for (const line of ['daemon', 'self-update /tmp/x', 'context list', 'list --server tcp://127.0.0.1:1']) {
			const { status } = await coreClient(home).post<unknown>('/v0/console/exec', { line });
			expect({ line, status }).toEqual({ line, status: 403 });
		}
		const empty = await coreClient(home).post<unknown>('/v0/console/exec', { line: '   ' });
		expect(empty.status).toBe(400);
	});

	it('lists the commands it will run, with no bare add', async () => {
		const { status, body } = await coreClient(home).get<{ commands: { path: string[] }[] }>('/v0/console/commands');
		expect(status).toBe(200);
		const names = (body?.commands ?? []).map((c) => c.path.join(' '));
		expect(names).toContain('install');
		expect(names).toContain('arrow add');
		expect(names).not.toContain('add');
		expect(names).not.toContain('daemon');
	});

	it('closes from the indicator', async () => {
		await $(INDICATOR).click();
		await browser.waitUntil(async () => (await $(PANEL).getAttribute('data-open')) === 'false', {
			timeout: 10_000,
			timeoutMsg: 'the console never closed',
		});
		await expect($(PANEL)).toHaveAttribute('inert');
	});
});
