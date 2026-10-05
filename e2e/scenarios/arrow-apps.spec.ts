import { browser, expect, $, $$ } from '@wdio/globals';
import { spawn, type ChildProcess } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

import { openRoute, waitForAppReady } from '../lib/app-ready';
import {
	ARROW_CSP,
	ChatClient,
	arrowHost,
	evidence,
	localEndpoint,
	lsRunDir,
	pidsByCommand,
	pidsByExe,
	rawRequest,
	runDir,
	screens,
	socketsInRunDir,
	ssListening,
	tcpListeners,
	uiPath,
	until,
	type Endpoint,
	type Listener,
} from '../lib/arrow-apps';
import { quiverCli } from '../lib/cli';
import { coreClient, getArrow, waitForCore, type ArrowDetailDTO } from '../lib/core-api';
import { E2E_TMP, quiverHome, selfInstalledCore } from '../lib/paths';
import { button, openArrowPage } from '../lib/ui';
import { wsConnect } from '../lib/ws-client';

// ARROW APPS, end to end: quiver.chat's real ARROW.md resolved from the box's
// GitHub stand-in, installed and executed by a real daemon, served on a unix
// socket the daemon provisions, proxied at /v0/ui, and shown in the real app
// in a sandboxed arrow-app:// iframe whose WebSocket frames cross the shell
// bridge. Three small fixture arrows (docker/e2e/fixtures/arrows) add a
// second and third origin, a late-binding interface and the 4-app cap.
//
// The `it`s run in order on one installation and share its state; each title
// names the check of the arrow-apps E2E brief it proves (A1..B12).

const CHAT_NS = process.env.QUIVER_E2E_CHAT_NS ?? '';
const ECHO_NS = process.env.QUIVER_E2E_ECHO_NS ?? '';
const STATIC_NS = (process.env.QUIVER_E2E_STATIC_NS ?? '').split(/\s+/).filter(Boolean);
const TCP_PORT = Number(process.env.QUIVER_E2E_TCP_PORT ?? 40299);

type Surface = { mode: string; path: string; ready: boolean };
type ActiveRun = NonNullable<ArrowDetailDTO['active_run']> & { surface?: Surface };

const nonce = crypto.randomBytes(3).toString('hex');

function surfaceOf(detail: ArrowDetailDTO | null): Surface | undefined {
	return (detail?.active_run as ActiveRun | null | undefined)?.surface;
}

const frame = (identity: string) => $(`iframe[title="${arrowHost(identity)}"]`);

async function classOf(identity: string): Promise<string> {
	return (await frame(identity).getAttribute('class')) ?? '';
}

async function toTop(): Promise<void> {
	await browser.switchToFrame(null);
}

/** Runs `script` inside the identity's iframe and comes back to the shell. */
async function inFrame<T>(identity: string, script: (...args: never[]) => T, ...args: unknown[]): Promise<T> {
	const el = await frame(identity);
	await el.waitForExist({ timeout: 30_000, timeoutMsg: `no iframe for ${identity} (${arrowHost(identity)})` });
	await browser.switchToFrame(el);
	try {
		return (await browser.execute(script as never, ...(args as never[]))) as T;
	} finally {
		await toTop();
	}
}

/** Like inFrame, for a script that settles a callback (the last argument). */
async function inFrameAsync<T>(identity: string, script: (...args: never[]) => void, ...args: unknown[]): Promise<T> {
	const el = await frame(identity);
	await el.waitForExist({ timeout: 30_000 });
	await browser.switchToFrame(el);
	try {
		return (await browser.executeAsync(script as never, ...(args as never[]))) as T;
	} finally {
		await toTop();
	}
}

async function frameText(identity: string): Promise<string> {
	return inFrame(identity, () => document.body?.textContent ?? '');
}

async function routePath(): Promise<string> {
	return decodeURIComponent(await browser.execute(() => window.location.pathname));
}

/** The app header (arrow-app-header.tsx): the row holding the Reload button. */
const header = () => $('//button[normalize-space(.)="Reload"]/../..');

async function headerText(): Promise<string> {
	return ((await (await header()).getAttribute('textContent')) ?? '').replace(/\s+/g, ' ').trim();
}

async function waitForSurface(home: string, identity: string, what: string, ready = true): Promise<ArrowDetailDTO> {
	return until(
		`${identity} never had ${what}`,
		async () => (await getArrow(home, identity)).body,
		(d) => surfaceOf(d)?.ready === ready,
		120_000,
		200
	) as Promise<ArrowDetailDTO>;
}

async function register(home: string, bare: string): Promise<string> {
	const added = await coreClient(home).post(`/v0/arrow/${encodeURIComponent(bare)}`);
	if (![200, 201].includes(added.status)) throw new Error(`adding ${bare} answered ${added.status}`);
	return (await getArrow(home, bare)).body!.namespace;
}

async function installAndExecute(home: string, identity: string): Promise<void> {
	const client = coreClient(home);
	const install = await client.post(`/v0/runtime/${encodeURIComponent(identity)}/install`, {});
	if (![200, 202].includes(install.status)) throw new Error(`install of ${identity} answered ${install.status}`);
	await until(
		`${identity} never installed`,
		async () => (await getArrow(home, identity)).body?.state ?? '',
		(s) => s === 'ready',
		180_000,
		500
	);
	const exec = await client.post(`/v0/runtime/${encodeURIComponent(identity)}/execute`, {});
	if (exec.status !== 202) throw new Error(`execute of ${identity} answered ${exec.status}`);
}

/** The Open button of the arrow-details hero (open-app-button.tsx). */
const openButton = () => button('Open');

/**
 * Types into a field of the current frame. WebKitWebDriver answers "Element is
 * not focusable" for inputs inside the arrow iframe (WebDriver's element send
 * keys), so when that happens the value goes in the way React's own tests do
 * it: the native value setter, then a bubbling `input` event. Recorded, so
 * the report can say which path ran.
 */
async function typeInto(selector: string, text: string): Promise<void> {
	const field = await $(selector);
	try {
		await field.setValue(text);
		typingPath.add('webdriver');
	} catch {
		await browser.execute(
			(sel: string, value: string) => {
				const el = document.querySelector(sel) as HTMLInputElement;
				Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!.call(el, value);
				el.dispatchEvent(new Event('input', { bubbles: true }));
			},
			selector,
			text
		);
		typingPath.add('script');
	}
}

/** Clicks in the current frame, through WebDriver when it can, else through the element's own click(). */
async function press(selector: string): Promise<void> {
	const el = await $(selector);
	try {
		await el.click();
	} catch {
		await browser.execute((sel: string) => {
			const found = document.evaluate(sel, document, null, XPathResult.FIRST_ORDERED_NODE_TYPE, null).singleNodeValue;
			(found as HTMLElement).click();
		}, selector);
	}
}

const typingPath = new Set<string>();

async function joinFromUi(identity: string, username: string): Promise<void> {
	const el = await frame(identity);
	await browser.switchToFrame(el);
	try {
		const name = await $('#username');
		await name.waitForExist({ timeout: 30_000, timeoutMsg: 'the chat never rendered its username field' });
		await typeInto('#username', username);
		await press('//button[normalize-space(.)="Connect"]');
		const input = await $('input[placeholder="Type your message..."]');
		await input.waitForExist({ timeout: 30_000 });
		await browser.waitUntil(async () => (await input.isEnabled()) === true, {
			timeout: 30_000,
			timeoutMsg: 'the chat message field never enabled (its WebSocket never opened)',
		});
	} finally {
		await toTop();
	}
}

async function sayFromUi(identity: string, text: string): Promise<void> {
	const el = await frame(identity);
	await browser.switchToFrame(el);
	try {
		await typeInto('input[placeholder="Type your message..."]', text);
		await press('//input[@placeholder="Type your message..."]/following-sibling::button');
	} finally {
		await toTop();
	}
}

async function waitForFrameText(identity: string, needle: string, timeout = 20_000): Promise<void> {
	let seen = '';
	try {
		await browser.waitUntil(
			async () => {
				seen = await frameText(identity);
				return seen.includes(needle);
			},
			{ timeout, interval: 300 }
		);
	} catch {
		throw new Error(`the ${identity} frame never showed "${needle}"; it shows: ${seen.slice(0, 600)}`);
	}
}

/** What a frame holds right now, read from inside it. */
async function frameState(
	identity: string
): Promise<{ href: string; origin: string; html: string; marker: unknown; timeOrigin: number }> {
	return inFrame(identity, () => ({
		timeOrigin: performance.timeOrigin,
		href: location.href,
		origin: location.origin,
		html: document.documentElement ? document.documentElement.outerHTML.slice(0, 600) : '',
		marker: (window as unknown as { __e2eMarker?: unknown }).__e2eMarker,
	}));
}

async function clickReload(): Promise<void> {
	await button('Reload').click();
}

describe('arrow apps: quiver.chat in the shell, end to end', () => {
	const home = process.env.QUIVER_E2E_HOME!;
	const local = localEndpoint(home);

	let chat = '';
	let echo = '';
	const statics: string[] = [];
	let chatPids: number[] = [];
	let executedAt = 0;
	let listenersBefore: Listener[] = [];
	let outside: ChatClient | null = null;
	let tcpDaemon: ChildProcess | null = null;
	let tcpLog = '';
	let tcpStop: (() => Promise<{ status: number }>) | null = null;

	before(async function () {
		if (!CHAT_NS || !ECHO_NS || STATIC_NS.length < 3) {
			throw new Error(
				'QUIVER_E2E_CHAT_NS, QUIVER_E2E_ECHO_NS and QUIVER_E2E_STATIC_NS are unset: this spec runs in the E2E box, ' +
					'with quiver.chat mounted (docker/e2e/README.md, "Arrow apps")'
			);
		}
		await waitForAppReady();
		await waitForCore(home);
		// Every origin a message reaches the shell from, as the shell's own
		// window sees it: the bridge only honours `arrowAppOrigin(host)`.
		await browser.execute(() => {
			const w = window as unknown as { __e2eOrigins: string[] };
			w.__e2eOrigins = [];
			window.addEventListener('message', (e) => w.__e2eOrigins.push(e.origin));
		});
	});

	after(() => {
		outside?.leave();
		tcpDaemon?.kill('SIGTERM');
		for (const pid of [...pidsByExe('quiver-chat-linux'), ...pidsByCommand(/server\.py /)]) {
			try {
				process.kill(pid, 'SIGKILL');
			} catch {
				/* already gone */
			}
		}
	});

	it('A0: quiver.chat\'s own ARROW.md, unmodified, passes this core\'s manifest validation', async () => {
		const manifest = fs.readFileSync(process.env.QUIVER_E2E_CHAT_MANIFEST ?? '', 'utf8');
		const res = await rawRequest(local, 'POST', `/v0/arrow/${encodeURIComponent(CHAT_NS)}/manifest/validate`, {}, manifest);
		evidence('A0.validate', { status: res.status, body: JSON.parse(res.body) });
		expect(res.status).toBe(200);
	});

	it('A1: installs and executes quiver.chat with the real CLI; the surface becomes {listen, /, ready}', async () => {
		const added = quiverCli(home, ['arrow', 'add', CHAT_NS]);
		evidence('A1.cli.add', added);
		expect(added.status).toBe(0);
		chat = (await getArrow(home, CHAT_NS)).body!.namespace;
		evidence('A1.identity', chat);

		const installed = quiverCli(home, ['install', chat]);
		evidence('A1.cli.install', { status: installed.status, stdout: installed.stdout.slice(-3000), stderr: installed.stderr.slice(-2000) });
		expect(installed.status).toBe(0);
		expect((await getArrow(home, chat)).body!.state).toBe('ready');

		listenersBefore = tcpListeners();
		evidence('A2.listeners.before', { listeners: listenersBefore, ss: ssListening() });

		const ran = quiverCli(home, ['run', chat, '--detach']);
		evidence('A1.cli.run', ran);
		expect(ran.status).toBe(0);
		executedAt = Date.now();

		const detail = await waitForSurface(home, chat, 'a ready surface');
		const runtime = await coreClient(home).get<{ state: string; active_run?: ActiveRun }>(
			`/v0/runtime/${encodeURIComponent(chat)}`
		);
		evidence('A1.detail', detail);
		evidence('A1.runtime', runtime);
		expect(runtime.body?.state).toBe('running');
		expect(runtime.body?.active_run?.surface).toEqual({ mode: 'listen', path: '/', ready: true });
		expect(surfaceOf(detail)).toEqual({ mode: 'listen', path: '/', ready: true });

		chatPids = await until('no quiver-chat process appeared', () => pidsByExe('quiver-chat-linux'), (p) => p.length > 0);
		evidence('A1.chat.pids', chatPids);
	});

	it('A2: opens no TCP listener for the chat; the run dir holds one <12hex>.sock and is 0700', async () => {
		const after = tcpListeners();
		const key = (l: Listener) => `${l.proto} ${l.address}:${l.port}`;
		const before = new Set(listenersBefore.map(key));
		const added = after.filter((l) => !before.has(key(l)));
		const chatOwned = after.filter((l) => l.pids.some((p) => chatPids.includes(p)));
		evidence('A2.listeners.after', { listeners: after, added, chatOwned, ss: ssListening() });
		expect(chatOwned).toEqual([]);
		expect(added.filter((l) => l.commands.some((c) => /quiver-chat/.test(c)))).toEqual([]);

		const dir = runDir(home);
		const mode = fs.statSync(dir).mode & 0o777;
		const sockets = socketsInRunDir(home);
		evidence('A2.rundir', { dir, mode: mode.toString(8), sockets, ls: lsRunDir(home) });
		expect(mode).toBe(0o700);
		expect(sockets.length).toBe(1);
		expect(sockets[0]).toMatch(/^[0-9a-f]{12}\.sock$/);
		expect(fs.statSync(path.join(dir, sockets[0])).isSocket()).toBe(true);
		// The address is derived from the namespace (surface.md, "Socket address").
		const expected = crypto.createHash('sha256').update(chat).digest('hex').slice(0, 12);
		evidence('A2.socket.derivation', { expected: `${expected}.sock`, bareExpected: crypto.createHash('sha256').update(CHAT_NS).digest('hex').slice(0, 12) });
	});

	it('A3: serves the chat HTML and a broadcasting WebSocket at /v0/ui, versioned and bare', async () => {
		const versioned = await rawRequest(local, 'GET', uiPath(chat));
		const bare = await rawRequest(local, 'GET', uiPath(CHAT_NS));
		evidence('A3.html', {
			versioned: { status: versioned.status, type: versioned.headers['content-type'], head: versioned.body.slice(0, 200) },
			bare: { status: bare.status, head: bare.body.slice(0, 200) },
		});
		for (const res of [versioned, bare]) {
			expect(res.status).toBe(200);
			expect(res.body).toContain('<html');
			expect(res.body).toContain('/_next/');
		}

		const alice = await ChatClient.join(local, chat, `curl-alice-${nonce}`);
		const bob = await ChatClient.join(local, CHAT_NS, `curl-bob-${nonce}`);
		await bob.waitFor('alice joining', (m) => m.content === `curl-bob-${nonce} joined the chat`);
		alice.say(`hello bob ${nonce}`);
		bob.say(`hello alice ${nonce}`);
		const atBob = await bob.waitFor("alice's message", (m) => m.content === `hello bob ${nonce}`);
		const atAlice = await alice.waitFor("bob's message", (m) => m.content === `hello alice ${nonce}`);
		evidence('A3.ws', { atBob, atAlice, alice: alice.messages, bob: bob.messages });
		expect(atBob.username).toBe(`curl-alice-${nonce}`);
		expect(atAlice.username).toBe(`curl-bob-${nonce}`);
		alice.leave();
		bob.leave();
	});

	it('B6: the details page Open button opens /app/<ns> with the header and a sandboxed arrow-app:// iframe', async () => {
		await openArrowPage(chat);
		await openButton().waitForExist({ timeout: 60_000, timeoutMsg: 'the chat details page has no Open button' });
		await browser.waitUntil(async () => (await openButton().isEnabled()) === true, {
			timeout: 30_000,
			timeoutMsg: 'Open never enabled although the surface is ready',
		});
		await screens('B6-details-open');
		await openButton().click();

		await browser.waitUntil(async () => (await routePath()) === `/app/${chat}`, {
			timeout: 15_000,
			timeoutMsg: 'Open did not navigate to /app/<namespace>',
		});
		await header().waitForExist({ timeout: 15_000 });
		await browser.waitUntil(async () => (await headerText()).includes('Running'), { timeout: 30_000 });
		const text = await headerText();
		const store = (await getArrow(home, chat)).body!;
		evidence('B6.header', { text, name: store.name, route: await routePath() });
		expect(text).toContain(store.name);
		expect(text).toMatch(/\d+\.\d+|develop/);
		for (const word of ['Running', 'Reload', 'Stop', 'Details']) expect(text).toContain(word);

		const el = await frame(chat);
		await el.waitForExist({ timeout: 30_000 });
		const attrs = {
			sandbox: await el.getAttribute('sandbox'),
			referrerpolicy: await el.getAttribute('referrerpolicy'),
			src: await el.getAttribute('src'),
			title: await el.getAttribute('title'),
			userAgent: await browser.execute(() => navigator.userAgent),
			shellOrigin: await browser.execute(() => location.origin),
		};
		evidence('B6.iframe', attrs);
		expect(attrs.sandbox).toBe('allow-scripts allow-same-origin');
		expect(attrs.referrerpolicy).toBe('no-referrer');
		expect(attrs.src).toMatch(/^arrow-app:\/\/[0-9a-f]{32}\.localhost\/$/);
		expect(attrs.src).toBe(`arrow-app://${arrowHost(chat)}/`);
		await screens('B6-app-open');
	});

	it('B7: the chat renders in the frame and talks both ways with a client outside the app', async () => {
		await waitForFrameText(chat, 'Join Chat', 30_000);
		const loaded = await inFrame(chat, () => ({
			origin: location.origin,
			href: location.href,
			scripts: Array.from(document.scripts).map((s) => s.src).filter(Boolean),
			styles: Array.from(document.querySelectorAll('link[rel="stylesheet"]')).map((l) => (l as HTMLLinkElement).href),
			shim: Array.from(document.scripts).some((s) => s.src.endsWith('/__arrow/shim.js')),
		}));
		evidence('B7.loaded', loaded);
		expect(loaded.origin).toBe(`arrow-app://${arrowHost(chat)}`);
		const next = [...loaded.scripts, ...loaded.styles].filter((r) => r.includes('/_next/'));
		expect(next.length).toBeGreaterThan(0);
		for (const r of next) expect(r.startsWith(`arrow-app://${arrowHost(chat)}/_next/`)).toBe(true);
		expect(loaded.shim).toBe(true);

		outside = await ChatClient.join(local, chat, `outside-${nonce}`);
		await joinFromUi(chat, `ui-alice-${nonce}`);
		await outside.waitFor('the UI user joining', (m) => m.content === `ui-alice-${nonce} joined the chat`);

		const fromUi = `from the app ${nonce}`;
		await sayFromUi(chat, fromUi);
		await waitForFrameText(chat, fromUi);
		const got = await outside.waitFor('the message typed in the app', (m) => m.content === fromUi);
		expect(got.username).toBe(`ui-alice-${nonce}`);

		const fromOutside = `from outside ${nonce}`;
		outside.say(fromOutside);
		await waitForFrameText(chat, fromOutside);

		const origins = await browser.execute(() => (window as unknown as { __e2eOrigins: string[] }).__e2eOrigins);
		evidence('B7.bridge.origins', { seenByShell: [...new Set(origins)], expected: `arrow-app://${arrowHost(chat)}` });
		expect(origins).toContain(`arrow-app://${arrowHost(chat)}`);
		evidence('B7.typing', [...typingPath]);
		await screens('B7-chat-roundtrip');
	});

	it('B8: keeps the same document alive across Details and another page, sockets included', async () => {
		const marker = `keep-${nonce}`;
		await inFrame(chat, (m: string) => {
			(window as unknown as { __e2eMarker: string }).__e2eMarker = m;
		}, marker);
		await browser.execute((title: string) => {
			(document.querySelector(`iframe[title="${title}"]`) as unknown as { __e2eKeep: string }).__e2eKeep = 'same';
		}, arrowHost(chat));
		const before = await frameText(chat);

		// B10 (Details): the header link goes to the arrow's own page.
		await $('//a[normalize-space(.)="Details"]').click();
		await browser.waitUntil(async () => (await routePath()) === `/arrow/${chat}`, {
			timeout: 15_000,
			timeoutMsg: 'Details did not navigate to /arrow/<namespace>',
		});
		evidence('B10.details', { route: await routePath() });
		await openRoute('/library');
		await browser.waitUntil(async () => (await routePath()) === '/library', { timeout: 15_000 });

		const hidden = await frame(chat);
		expect(await hidden.isExisting()).toBe(true);
		expect(await hidden.getAttribute('class')).toContain('hidden');
		expect(await hidden.isDisplayed()).toBe(false);

		const whileHidden = `while hidden ${nonce}`;
		outside!.say(whileHidden);
		await new Promise((r) => setTimeout(r, 1500));

		await openArrowPage(chat);
		await openButton().waitForExist({ timeout: 30_000 });
		await openButton().click();
		await browser.waitUntil(async () => (await routePath()) === `/app/${chat}`, { timeout: 15_000 });

		const sameElement = await browser.execute(
			(title: string) => (document.querySelector(`iframe[title="${title}"]`) as unknown as { __e2eKeep?: string }).__e2eKeep,
			arrowHost(chat)
		);
		const state = await frameState(chat);
		const after = await frameText(chat);
		evidence('B8.keepalive', { sameElement, marker: state.marker, beforeLength: before.length, after: after.slice(-400) });
		expect(sameElement).toBe('same');
		expect(state.marker).toBe(marker);
		expect(after).toContain(`from the app ${nonce}`);
		expect(after).toContain(`from outside ${nonce}`);
		expect(after).toContain(whileHidden);
		expect(outside!.messages.some((m) => m.content === `ui-alice-${nonce} left the chat`)).toBe(false);
		await screens('B8-after-keepalive');
	});

	it('B9a: the frame has no Tauri globals, can not reach quiver:// or the daemon API, and carries ARROW_CSP', async () => {
		const globals = await inFrame(chat, () => {
			const w = window as unknown as Record<string, unknown> & { webkit?: { messageHandlers?: Record<string, unknown> } };
			let parent = 'readable';
			try {
				void (window.parent as Window).document.title;
			} catch (e) {
				parent = `blocked: ${(e as Error).name}`;
			}
			document.cookie = 'e2e=1';
			return {
				tauriInternals: typeof w.__TAURI_INTERNALS__,
				tauri: typeof w.__TAURI__,
				quiver: typeof w.__QUIVER__,
				ipc: typeof w.ipc,
				webkitHandlers: w.webkit?.messageHandlers ? Object.keys(w.webkit.messageHandlers) : null,
				ipcHandler: typeof w.webkit?.messageHandlers?.ipc,
				parent,
				cookieAfterWrite: document.cookie,
				isSecureContext: window.isSecureContext,
				origin: location.origin,
			};
		});
		evidence('B9a.globals', globals);
		expect(globals.tauriInternals).toBe('undefined');
		expect(globals.tauri).toBe('undefined');
		expect(globals.quiver).toBe('undefined');
		expect(globals.parent).toMatch(/^blocked/);

		const probes = await inFrameAsync<Record<string, unknown>>(chat, (done: (v: unknown) => void) => {
			const settle = (p: Promise<unknown>) =>
				p.then(
					(v) => ({ resolved: v }),
					(e) => ({ rejected: String(e) })
				);
			void Promise.all([
				settle(fetch('quiver://localhost/v0/health').then((r) => r.status)),
				settle(fetch('/v0/arrow').then(async (r) => ({ status: r.status, body: (await r.text()).slice(0, 200) }))),
				settle(
					fetch(location.href).then(async (r) => ({
						status: r.status,
						csp: r.headers.get('content-security-policy'),
						nosniff: r.headers.get('x-content-type-options'),
						referrer: r.headers.get('referrer-policy'),
						setCookie: r.headers.get('set-cookie'),
						shim: (await r.text()).includes('<script src="/__arrow/shim.js"></script>'),
					}))
				),
				settle(fetch('arrow-app://deadbeefdeadbeefdeadbeefdeadbeef.localhost/').then((r) => r.status)),
				settle(fetch('/__arrow/shim.js').then((r) => r.status)),
				settle(fetch('/__arrow/anything-else').then((r) => r.status)),
				settle(fetch('/%2e%2e/v0/health').then((r) => r.status)),
			]).then(([quiverFetch, ownApi, page, foreign, shim, otherArrowPath, dotdot]) =>
				done({ quiverFetch, ownApi, page, foreign, shim, otherArrowPath, dotdot })
			);
		});
		evidence('B9a.probes', probes);
		expect(probes.quiverFetch).toHaveProperty('rejected');
		const ownApi = (probes.ownApi as { resolved?: { status: number; body: string } }).resolved;
		expect(ownApi).toBeDefined();
		expect(ownApi!.body).not.toContain('"success"');
		expect(ownApi!.status).toBe(404);
		const page = (probes.page as { resolved?: { csp: string; nosniff: string; referrer: string; shim: boolean } }).resolved!;
		expect(page.csp).toBe(ARROW_CSP);
		expect(page.nosniff).toBe('nosniff');
		expect(page.referrer).toBe('no-referrer');
		expect(page.shim).toBe(true);
		expect(probes.foreign).toHaveProperty('rejected');
		expect(probes.shim).toEqual({ resolved: 200 });
		expect(probes.otherArrowPath).toEqual({ resolved: 404 });
		expect(globals.cookieAfterWrite).not.toContain('e2e=1');
	});

	it('A3/B6: a late-binding app shows Starting..., keeps Open disabled until ready, and never sees Authorization or Cookie', async () => {
		echo = await register(home, ECHO_NS);
		const client = coreClient(home);
		expect([200, 202]).toContain((await client.post(`/v0/runtime/${encodeURIComponent(echo)}/install`, {})).status);
		await until('the echo app never installed', async () => (await getArrow(home, echo)).body?.state ?? '', (s) => s === 'ready', 120_000, 500);

		await openArrowPage(echo);
		await browser.waitUntil(async () => (await routePath()) === `/arrow/${echo}`, { timeout: 15_000 });
		expect((await client.post(`/v0/runtime/${encodeURIComponent(echo)}/execute`, {})).status).toBe(202);

		await openButton().waitForExist({ timeout: 30_000, timeoutMsg: 'the echo app never showed Open' });
		const disabledWhileStarting = !(await openButton().isEnabled());
		const notReady = surfaceOf((await getArrow(home, echo)).body);
		await screens('B6-echo-open-disabled');
		evidence('B6.echo.disabled', { disabledWhileStarting, surface: notReady });
		expect(notReady).toEqual({ mode: 'listen', path: '/', ready: false });
		expect(disabledWhileStarting).toBe(true);

		await openRoute(`/app/${echo}`);
		await header().waitForExist({ timeout: 15_000 });
		const starting = await headerText();
		const frameWhileStarting = await frame(echo).isExisting();
		evidence('B6.echo.starting', { header: starting, frameWhileStarting });
		expect(starting).toContain('Starting...');
		expect(frameWhileStarting).toBe(false);
		await screens('B6-echo-starting');

		await waitForSurface(home, echo, 'a ready surface');
		await browser.waitUntil(async () => (await headerText()).includes('Running'), { timeout: 30_000 });
		await frame(echo).waitForExist({ timeout: 30_000 });
		await waitForFrameText(echo, 'E2E Echo App');

		const viaDaemon = await rawRequest(local, 'GET', uiPath(echo, '/headers?via=daemon'), {
			Authorization: 'Bearer leaked-token',
			Cookie: 'session=leaked',
			'X-E2E-Kept': 'yes',
		});
		const seenDaemon = JSON.parse(viaDaemon.body) as { headers: Record<string, string>; path: string };
		const viaApp = await inFrameAsync<{ headers: Record<string, string> }>(echo, (done: (v: unknown) => void) => {
			fetch('/headers?via=app', { headers: { 'X-E2E-Kept': 'yes' } })
				.then((r) => r.json())
				.then(done, (e) => done({ error: String(e) }));
		});
		evidence('A3.headers', { viaDaemon: seenDaemon, viaApp });
		expect(seenDaemon.headers['x-e2e-kept']).toBe('yes');
		expect(seenDaemon.headers.authorization).toBeUndefined();
		expect(seenDaemon.headers.cookie).toBeUndefined();
		expect(seenDaemon.path).toBe('/headers?via=daemon');
		for (const h of ['authorization', 'cookie', 'origin', 'referer']) expect(viaApp.headers[h]).toBeUndefined();
	});

	it('B11: static apps get their own origins and storage, stay mounted hidden, and switch without reloading', async () => {
		for (const bare of STATIC_NS) statics.push(await register(home, bare));
		for (const identity of statics) await installAndExecute(home, identity);
		for (const identity of statics) {
			const d = await waitForSurface(home, identity, 'a ready static surface');
			expect(surfaceOf(d)?.mode).toBe('static');
		}
		const [s1, s2, s3] = statics;

		// Open order echo, chat, s1, s2 makes echo the least recently used.
		await openRoute(`/app/${chat}`);
		await browser.waitUntil(async () => !(await classOf(chat)).includes('hidden'), { timeout: 15_000 });
		for (const identity of [s1, s2]) {
			await openRoute(`/app/${identity}`);
			await frame(identity).waitForExist({ timeout: 30_000 });
			await waitForFrameText(identity, `E2E Static App ${statics.indexOf(identity) + 1}`);
		}

		await inFrame(s1, () => {}); // s1 is hidden now; WebDriver may still enter it
		const fromS1 = await inFrame(s1, (m: string) => {
			localStorage.setItem('e2e-key', 'written by app 1');
			(window as unknown as { __e2eMarker: string }).__e2eMarker = m;
			return { origin: location.origin, stored: localStorage.getItem('e2e-key') };
		}, `s1-${nonce}`);
		const fromS2 = await inFrame(s2, () => ({ origin: location.origin, stored: localStorage.getItem('e2e-key') }));
		evidence('B11.isolation', { fromS1, fromS2 });
		expect(fromS1.stored).toBe('written by app 1');
		expect(fromS2.stored).toBeNull();
		expect(fromS1.origin).not.toBe(fromS2.origin);
		expect(fromS2.origin).toBe(`arrow-app://${arrowHost(s2)}`);

		const classes = {
			chat: await frame(chat).getAttribute('class'),
			echo: await frame(echo).getAttribute('class'),
			s1: await frame(s1).getAttribute('class'),
			s2: await frame(s2).getAttribute('class'),
		};
		evidence('B11.hidden', classes);
		expect(classes.s2).not.toContain('hidden');
		for (const c of [classes.chat, classes.echo, classes.s1]) expect(c).toContain('hidden');
		expect(await frame(s2).isDisplayed()).toBe(true);
		expect(await frame(s1).isDisplayed()).toBe(false);
		await screens('B11-static-app-2');

		const beforeSwitch = { s1: await frameState(s1), chat: await frameState(chat) };
		const domOrderBefore = await $$('iframe').map((f) => f.getAttribute('title'));
		await openRoute(`/app/${s1}`);
		await browser.waitUntil(async () => !(await classOf(s1)).includes('hidden'), { timeout: 15_000 });
		const afterSwitch = { s1: await frameState(s1), chat: await frameState(chat) };
		const domOrderAfter = await $$('iframe').map((f) => f.getAttribute('title'));
		evidence('B11.switch', {
			domOrderBefore,
			domOrderAfter,
			s1: { markerBefore: beforeSwitch.s1.marker, markerAfter: afterSwitch.s1.marker, timeOriginBefore: beforeSwitch.s1.timeOrigin, timeOriginAfter: afterSwitch.s1.timeOrigin },
			chat: { markerAfter: afterSwitch.chat.marker, timeOriginBefore: beforeSwitch.chat.timeOrigin, timeOriginAfter: afterSwitch.chat.timeOrigin },
			outsideSawLeaves: outside?.messages.filter((m) => m.content.endsWith('left the chat')),
		});
		expect(afterSwitch.s1.timeOrigin).toBe(beforeSwitch.s1.timeOrigin);
		expect(afterSwitch.s1.marker).toBe(`s1-${nonce}`);
		expect(afterSwitch.chat.marker).toBe(`keep-${nonce}`);
	});

	it('B11b: opening a fifth app evicts the least recently used hidden one (cap 4)', async () => {
		const [s1, s2, s3] = statics;
		await openRoute(`/app/${s3}`);
		await frame(s3).waitForExist({ timeout: 30_000 });
		await waitForFrameText(s3, 'E2E Static App 3');
		await browser.waitUntil(async () => !(await frame(echo).isExisting()), {
			timeout: 15_000,
			timeoutMsg: 'opening a fifth app did not evict the least recently used one',
		});
		const titles = await $$('iframe').map((f) => f.getAttribute('title'));
		evidence('B11.cap', { titles, expected: [chat, s1, s2, s3].map(arrowHost), evicted: arrowHost(echo) });
		expect([...titles].sort()).toEqual([chat, s1, s2, s3].map(arrowHost).sort());
		expect(surfaceOf((await getArrow(home, echo)).body)?.ready).toBe(true);
		await screens('B11-static-app-3-after-cap');
	});

	it('B9c: a chat frame navigated to another arrow origin loses the bridge; B10: Reload remounts a working chat', async () => {
		const [s1] = statics;
		await openRoute(`/app/${chat}`);
		await browser.waitUntil(async () => !(await classOf(chat)).includes('hidden'), { timeout: 15_000 });
		// A fresh chat document with a fresh user, so this check does not depend on
		// the ones before it having kept the first one alive.
		outside ??= await ChatClient.join(local, chat, `outside-${nonce}`);
		await clickReload();
		await waitForFrameText(chat, 'Join Chat', 30_000);
		await joinFromUi(chat, `ui-dave-${nonce}`);
		await outside.waitFor('the UI user joining', (m) => m.content === `ui-dave-${nonce} joined the chat`);

		await inFrame(chat, (target: string) => {
			window.location.href = target;
		}, `arrow-app://${arrowHost(s1)}/`);
		const left = await outside!.waitFor(
			'the chat socket closing when its frame left for another origin',
			(m) => m.content === `ui-dave-${nonce} left the chat`,
			15_000
		);
		await waitForFrameText(chat, 'E2E Static App 1');
		const foreign = await frameState(chat);
		await inFrame(chat, (url: string) => {
			try {
				new WebSocket(url);
			} catch {
				/* the shim may refuse it outright */
			}
		}, 'ws://localhost/ws?username=intruder');
		await new Promise((r) => setTimeout(r, 3000));
		const intruder = outside!.messages.some((m) => m.content.startsWith('intruder'));
		evidence('B9c.foreign', { left, foreign, intruderJoined: intruder });
		expect(foreign.origin).toBe(`arrow-app://${arrowHost(s1)}`);
		expect(intruder).toBe(false);
		await screens('B9c-chat-frame-on-foreign-origin');

		await browser.execute((title: string) => {
			(document.querySelector(`iframe[title="${title}"]`) as unknown as { __e2eKeep: string }).__e2eKeep = 'old';
		}, arrowHost(chat));
		await clickReload();
		await waitForFrameText(chat, 'Join Chat', 30_000);
		const remounted = await browser.execute(
			(title: string) => (document.querySelector(`iframe[title="${title}"]`) as unknown as { __e2eKeep?: string }).__e2eKeep ?? null,
			arrowHost(chat)
		);
		const fresh = await frameState(chat);
		evidence('B10.reload', { remounted, fresh });
		expect(remounted).toBeNull();
		expect(fresh.origin).toBe(`arrow-app://${arrowHost(chat)}`);
		// WebDriver serialises undefined as null.
		expect(fresh.marker ?? null).toBeNull();

		await joinFromUi(chat, `ui-carol-${nonce}`);
		await outside!.waitFor('the reloaded UI joining', (m) => m.content === `ui-carol-${nonce} joined the chat`);
		await sayFromUi(chat, `after reload ${nonce}`);
		await outside!.waitFor('a message from the reloaded UI', (m) => m.content === `after reload ${nonce}`);
		outside!.say(`to the reloaded UI ${nonce}`);
		await waitForFrameText(chat, `to the reloaded UI ${nonce}`);
		await screens('B10-after-reload');
	});

	it('B9b: a frame navigating itself to quiver:// does not render daemon content as a quiver:// document', async () => {
		const targets = [`quiver://localhost${uiPath(chat)}`, 'quiver://localhost/v0/health', 'quiver://localhost/v0/arrow'];
		const results: Record<string, unknown>[] = [];
		for (const target of targets) {
			await inFrame(chat, (url: string) => {
				window.location.href = url;
			}, target);
			await new Promise((r) => setTimeout(r, 3000));
			let state: Record<string, unknown>;
			try {
				state = await frameState(chat);
			} catch (e) {
				state = { error: String(e) };
			}
			const shellView = await browser.execute((title: string) => {
				const f = document.querySelector(`iframe[title="${title}"]`) as HTMLIFrameElement | null;
				try {
					return { href: f?.contentWindow?.location.href ?? null };
				} catch (e) {
					return { blocked: String(e) };
				}
			}, arrowHost(chat));
			results.push({ target, state, shellView });
			await screens(`B9b-quiver-navigation-${results.length}`);
			await clickReload();
			await waitForFrameText(chat, 'Join Chat', 30_000);
		}
		evidence('B9b.quiver-navigation', results);
		for (const r of results) {
			const state = r.state as { origin?: string; html?: string };
			expect(state.origin).not.toBe('quiver://localhost');
			expect(state.html ?? '').not.toContain('/_next/');
			expect(state.html ?? '').not.toContain('"success"');
		}
	});

	it('the chat is still running past 60 seconds with the same process', async () => {
		const elapsed = Date.now() - executedAt;
		if (elapsed < 65_000) await new Promise((r) => setTimeout(r, 65_000 - elapsed));
		const runtime = await coreClient(home).get<{ state: string }>(`/v0/runtime/${encodeURIComponent(chat)}`);
		const pids = pidsByExe('quiver-chat-linux');
		evidence('survive', { seconds: Math.round((Date.now() - executedAt) / 1000), state: runtime.body?.state, pids, chatPids });
		expect(runtime.body?.state).toBe('running');
		expect(pids).toEqual(chatPids);
	});

	it('B10/A5: Stop in the header ends the chat: frame gone, socket gone, process gone, /v0/ui answers 503', async () => {
		await openRoute(`/app/${chat}`);
		await header().waitForExist({ timeout: 15_000 });
		await button('Stop').click();

		await frame(chat).waitForExist({ reverse: true, timeout: 60_000, timeoutMsg: 'the chat iframe stayed after Stop' });
		await $('//p[normalize-space(.)="This arrow is not running, so it has no interface to show."]').waitForExist({
			timeout: 30_000,
			timeoutMsg: 'the app route never showed its not-running state',
		});
		await screens('B10-after-stop');

		const gone = await until('the chat process outlived Stop', () => pidsByExe('quiver-chat-linux'), (p) => p.length === 0, 30_000);
		const echoSocket = crypto.createHash('sha256').update(echo).digest('hex').slice(0, 12);
		const sockets = await until(
			'the chat socket file outlived Stop',
			() => socketsInRunDir(home).filter((s) => s !== `${echoSocket}.sock`),
			(s) => s.length === 0,
			30_000
		);
		const ui = await rawRequest(local, 'GET', uiPath(chat));
		const runtime = await coreClient(home).get<{ state: string; active_run?: ActiveRun | null }>(
			`/v0/runtime/${encodeURIComponent(chat)}`
		);
		evidence('A5.stopped', {
			pids: gone,
			socketsLeft: sockets,
			ls: lsRunDir(home),
			ui: { status: ui.status, body: ui.body.slice(0, 200) },
			runtime: runtime.body,
			outsideClosed: outside?.closed,
		});
		expect(ui.status).toBe(503);
		expect(runtime.body?.active_run?.surface).toBeUndefined();
		expect(['ready', 'stopped']).toContain(runtime.body?.state);
	});

	it('A4: over tcp:// the /v0/ui route needs the paired bearer token, WebSocket upgrades included', async () => {
		const tcpHome = path.join(E2E_TMP, 'arrow-apps-tcp');
		fs.rmSync(tcpHome, { recursive: true, force: true });
		fs.mkdirSync(quiverHome(tcpHome), { recursive: true });
		fs.writeFileSync(path.join(quiverHome(tcpHome), 'config.yaml'), 'config:\n  arrows:\n    version_check_ttl: 1s\n');
		tcpLog = path.join(process.env.QUIVER_E2E_RESULTS ?? tcpHome, 'arrow-apps', 'tcp-daemon.log');
		const log = fs.openSync(tcpLog, 'a');
		tcpDaemon = spawn(selfInstalledCore(home), ['daemon', '--host', `tcp://127.0.0.1:${TCP_PORT}`], {
			env: { ...process.env, HOME: tcpHome, QUIVER_HOME: quiverHome(tcpHome) },
			stdio: ['ignore', log, log],
		});
		const tcp: Endpoint = { host: '127.0.0.1', port: TCP_PORT };
		await until('the tcp daemon never answered', async () => (await rawRequest(tcp, 'GET', '/v0/health').catch(() => null))?.status ?? 0, (s) => s === 200, 60_000, 500);

		const code = JSON.parse((await rawRequest(tcp, 'POST', '/v0/auth/pairing')).body).data.code as string;
		const redeemed = await rawRequest(tcp, 'POST', '/v0/auth/pairing/redeem', {}, { code, device_id: `e2e-${nonce}`, label: 'arrow-apps e2e' });
		const token = JSON.parse(redeemed.body).data.token as string;
		expect(token).toBeTruthy();
		const auth = { Authorization: `Bearer ${token}` };

		const add = await rawRequest(tcp, 'POST', `/v0/arrow/${encodeURIComponent(CHAT_NS)}`, auth);
		expect([200, 201]).toContain(add.status);
		const identity = JSON.parse((await rawRequest(tcp, 'GET', `/v0/arrow/${encodeURIComponent(CHAT_NS)}`, auth)).body).data.namespace as string;
		expect([200, 202]).toContain((await rawRequest(tcp, 'POST', `/v0/runtime/${encodeURIComponent(identity)}/install`, auth, {})).status);
		await until('the tcp daemon never installed chat', async () => JSON.parse((await rawRequest(tcp, 'GET', `/v0/arrow/${encodeURIComponent(identity)}`, auth)).body).data?.state, (s) => s === 'ready', 120_000, 500);
		expect((await rawRequest(tcp, 'POST', `/v0/runtime/${encodeURIComponent(identity)}/execute`, auth, {})).status).toBe(202);
		// Not `ready`: the bearer gate is what this check is about, and the
		// route proxies as soon as the surface exists. Whether the readiness
		// probe ever recorded `ready` here is kept as evidence (see the report's
		// surface-probe race).
		await until(
			'the tcp daemon never had a chat surface answering through /v0/ui',
			async () => (await rawRequest(tcp, 'GET', uiPath(identity), auth)).status,
			(st) => st === 200,
			60_000,
			300
		);
		const tcpSurface = JSON.parse((await rawRequest(tcp, 'GET', `/v0/arrow/${encodeURIComponent(identity)}`, auth)).body).data?.active_run?.surface;

		const noToken = await rawRequest(tcp, 'GET', uiPath(identity));
		const badToken = await rawRequest(tcp, 'GET', uiPath(identity), { Authorization: 'Bearer nope' });
		const withToken = await rawRequest(tcp, 'GET', uiPath(identity), auth);
		const wsNoToken = await wsConnect({ ...tcp, path: uiPath(identity, '/ws?username=anon') });
		const wsBadToken = await wsConnect({ ...tcp, path: uiPath(identity, '/ws?username=anon'), headers: { Authorization: 'Bearer nope' } });
		wsNoToken.ws?.close();
		wsBadToken.ws?.close();
		const a = await ChatClient.join(tcp, identity, `tcp-a-${nonce}`, auth);
		const b = await ChatClient.join(tcp, CHAT_NS, `tcp-b-${nonce}`, auth);
		await b.waitFor('a joining', (m) => m.content === `tcp-b-${nonce} joined the chat`);
		a.say(`over tcp ${nonce}`);
		const got = await b.waitFor('a message over tcp', (m) => m.content === `over tcp ${nonce}`);
		a.leave();
		b.leave();
		evidence('A4.tcp', {
			noToken: { status: noToken.status, body: noToken.body.slice(0, 200) },
			badToken: badToken.status,
			withToken: { status: withToken.status, head: withToken.body.slice(0, 80) },
			wsNoToken: { status: wsNoToken.status, body: wsNoToken.body },
			wsBadToken: { status: wsBadToken.status },
			got,
			tcpSurface,
		});
		expect(noToken.status).toBe(401);
		expect(badToken.status).toBe(401);
		expect(withToken.status).toBe(200);
		expect(withToken.body).toContain('<html');
		expect(wsNoToken.status).toBe(401);
		expect(wsBadToken.status).toBe(401);

		tcpStop = async () => {
			const stop = await rawRequest(tcp, 'POST', `/v0/runtime/${encodeURIComponent(identity)}/stop`, auth, {});
			return { status: stop.status };
		};
	});

	it('A4b: stopping the arrow on the tcp daemon ends its process', async () => {
		if (!tcpStop) throw new Error('A4 did not get as far as a running chat on the tcp daemon');
		const stop = await tcpStop();
		let left: number[] = [];
		try {
			left = await until('the tcp chat outlived its stop', () => pidsByExe('quiver-chat-linux'), (p) => p.length === 0, 30_000);
		} catch {
			left = pidsByExe('quiver-chat-linux');
		}
		evidence('A4b.stop', { stop, chatPidsLeft: left });
		expect(left).toEqual([]);
	});

	it('core: no runtime command was lost to an event-store version conflict during this run', () => {
		const logs = [path.join(quiverHome(home), 'logs', 'Quiver.log'), tcpLog];
		const lost: string[] = [];
		for (const file of logs) {
			if (!file || !fs.existsSync(file)) continue;
			for (const line of fs.readFileSync(file, 'utf8').split('\n')) {
				if (line.includes('version conflict')) lost.push(`${path.basename(path.dirname(file))}/${path.basename(file)}: ${line}`);
			}
		}
		evidence('core.version-conflicts', lost);
		expect(lost).toEqual([]);
	});
});
