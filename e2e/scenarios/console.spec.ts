import fs from 'node:fs';
import path from 'node:path';

import { browser, $, expect } from '@wdio/globals';

import { waitForAppReady } from '../lib/app-ready';
import { coreClient, waitForCore } from '../lib/core-api';
import { quiverHome } from '../lib/paths';
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

/** How many error-toned notes the console shows right now. */
async function errorNotes(): Promise<number> {
	return browser.execute(() => document.querySelectorAll('[data-slot="console-note"][data-tone="error"]').length);
}

/** Where the log pane is scrolled and which rows it has rendered: what a failure needs to be understood. */
async function logGeometry(): Promise<string> {
	return browser.execute(() => {
		const el = document.querySelector('[data-slot="console-log"]') as HTMLElement | null;
		if (!el) return 'no log pane';
		const idx = Array.from(el.querySelectorAll('[data-index]'), (n) => Number(n.getAttribute('data-index')));
		const inner = el.firstElementChild as HTMLElement | null;
		return JSON.stringify({
			scrollTop: el.scrollTop,
			scrollHeight: el.scrollHeight,
			clientHeight: el.clientHeight,
			innerHeight: inner?.style.height,
			rendered: idx.length,
			firstIndex: idx.length ? Math.min(...idx) : null,
			lastIndex: idx.length ? Math.max(...idx) : null,
		});
	});
}

interface LogRow {
	level: string;
	msg: string;
	fields: Record<string, string>;
}

/**
 * The log lines rendered right now, read by structure. The pane is virtualized, so
 * these are the rows around the viewport -- the newest, while it follows the tail --
 * and a field is a key and a value in two elements, with the `=` drawn by CSS: text
 * matching on `textContent` cannot see it.
 */
async function logRows(): Promise<LogRow[]> {
	return browser.execute(() =>
		Array.from(document.querySelectorAll('[data-slot="log-row"]'), (row) => {
			const cells = row.querySelectorAll(':scope > button > span');
			const message = cells[2];
			const fields: Record<string, string> = {};
			message?.querySelectorAll(':scope > span.inline-block').forEach((field) => {
				const [key, value] = Array.from(field.children, (c) => c.textContent ?? '');
				fields[key] = value ?? '';
			});
			return {
				level: row.getAttribute('data-level') ?? '',
				msg: message?.firstElementChild?.textContent ?? '',
				fields,
			};
		})
	);
}

/**
 * The daemon's own audit record of commands it ran: `msg=exec` with a device, the
 * resolved command (`quiver arrow remove`, not the line typed) and a code. The row
 * draws no component.
 */
async function waitForAudit(command: string, code: number, timeout = 60_000): Promise<void> {
	let rows: LogRow[] = [];
	const audited = (r: LogRow): boolean =>
		r.msg === 'exec' &&
		(r.fields.command ?? '').replace(/^quiver /, '') === command &&
		r.fields.code === String(code);
	try {
		await browser.waitUntil(
			async () => {
				rows = await logRows();
				return rows.some(audited);
			},
			{ timeout, interval: 300 }
		);
	} catch {
		const seen = rows.filter((r) => r.msg === 'exec').map((r) => JSON.stringify(r.fields));
		throw new Error(
			`the log never showed the audit record of "${command}" (code ${code}); log pane ${await logGeometry()}; exec rows: ${seen.join(' | ') || 'none'}`
		);
	}
}

/**
 * How many times the daemon's own log file says it ran `command` to `code`. The
 * pane only renders the rows near its viewport, so counting rows there depends on
 * how much else was logged; the file does not.
 */
function daemonAudits(home: string, command: string, code: number): number {
	const log = fs.readFileSync(path.join(quiverHome(home), 'logs', 'Quiver.log'), 'utf8');
	return log.split('\n').filter((line) => {
		try {
			const record = JSON.parse(line);
			return (
				record.msg === 'exec' &&
				String(record.command).replace(/^quiver /, '') === command &&
				record.code === code
			);
		} catch {
			return false;
		}
	}).length;
}

/** The notes the console itself prints (a refusal, a non-zero exit). */
async function noteTexts(): Promise<string[]> {
	return browser.execute(() =>
		Array.from(document.querySelectorAll('[data-slot="console-note"]'), (n) => n.textContent ?? '')
	);
}

async function waitForNotes(pattern: RegExp, atLeast: number, timeout = 30_000): Promise<void> {
	let seen: string[] = [];
	try {
		await browser.waitUntil(
			async () => {
				seen = await noteTexts();
				return seen.filter((t) => pattern.test(t)).length >= atLeast;
			},
			{ timeout, interval: 300 }
		);
	} catch {
		throw new Error(
			`the console never showed ${atLeast} note(s) matching ${pattern}; notes: ${JSON.stringify(seen)}; log pane ${await logGeometry()}`
		);
	}
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
		throw new Error(
			`the console never showed ${pattern}; log pane ${await logGeometry()}; it shows:\n${seen.slice(-600)}`
		);
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

	it("opens from the indicator and shows the daemon's own log, typed", async () => {
		await openConsole();
		await $('[data-slot="log-row"]').waitForExist({ timeout: 30_000, timeoutMsg: 'the daemon log never appeared' });

		// Read in the page itself: WebdriverIO types a list of elements as promises all
		// the way down, and one round trip is also all the rows that are rendered now.
		const levels = await browser.execute(() =>
			Array.from(document.querySelectorAll('[data-slot="log-row"]'), (row) => row.getAttribute('data-level'))
		);
		expect(levels.length).toBeGreaterThan(0);
		// Every row carries a level the styling keys off.
		for (const level of levels.slice(0, 20)) expect(['debug', 'info', 'warn', 'error']).toContain(level);
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
		// The daemon records the run, with its exit code, in the log the console shows.
		await waitForAudit('list', 0);
		// A clean exit says nothing extra; a failure ends in an alert line.
		expect(await errorNotes()).toBe(0);
	});

	it("refuses what the daemon does not offer, naming it in the daemon's own words", async () => {
		await run('daemon');
		await waitForNotes(/^Refused \(403\): command "daemon" is not available in the console$/, 1);
		await run('add github.com/char2cs/crowbar');
		await waitForNotes(/^Refused \(403\): command "add" is not available in the console$/, 1);
		await shot('console-refused');
	});

	it("refuses a line with quoting in the daemon's own words: the console does no quoting of its own", async () => {
		await run('install "github.com/char2cs/crowbar"');
		await waitForNotes(/^Refused \(400\): line must be 1 to 1024 bytes .*quoting is not supported$/, 1);
	});

	it('accepts --server and runs the command on this daemon all the same', async () => {
		// If the flag were honoured the command would dial 127.0.0.1:1 and fail (code 3);
		// it runs here and exits 0, and the daemon audits one more run.
		const before = daemonAudits(home, 'version', 0);
		await run('version --server tcp://127.0.0.1:1');
		await browser.waitUntil(() => daemonAudits(home, 'version', 0) === before + 1, {
			timeout: 60_000,
			timeoutMsg: 'the daemon never audited the --server run of "version" at code 0',
		});
	});

	it('never waits on a confirmation: a destructive command without --yes refuses at once', async () => {
		await run('arrow remove github.com/example/not-there');
		await waitForConsoleText(/requires --yes/);
		expect(await errorNotes()).toBeGreaterThan(0);
	});

	it('logs the commands it ran, as the daemon records them', async () => {
		await waitForAudit('version', 0);
		await waitForAudit('arrow remove', 2);
	});

	it('refuses, on the wire, what the console must never run', async () => {
		for (const line of ['daemon', 'self-update /tmp/x', 'context list']) {
			const { status } = await coreClient(home).post<unknown>('/v0/console/exec', { line });
			expect({ line, status }).toEqual({ line, status: 403 });
		}
	});

	it('rejects, on the wire, a line with quoting, shell characters, or nothing in it', async () => {
		for (const line of [
			'install "a b"',
			"install 'a'",
			'list; id',
			'list | cat',
			'install $(id)',
			'   ',
			'x'.repeat(1025),
		]) {
			const { status } = await coreClient(home).post<unknown>('/v0/console/exec', { line });
			expect({ line: line.slice(0, 40), status }).toEqual({ line: line.slice(0, 40), status: 400 });
		}
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
