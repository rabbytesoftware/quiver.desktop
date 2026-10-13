import { browser } from '@wdio/globals';
import { spawn, type ChildProcess } from 'node:child_process';
import fs from 'node:fs';
import net from 'node:net';
import path from 'node:path';

import { APP_BINARY, E2E_TMP, homeForSpec, quiverHome, selfInstalledCore } from './lib/paths';

/**
 * WHY THERE IS NO `tauri-driver.conf.json` IN THIS DIRECTORY.
 *
 * The task brief allowed for one "or equivalent -- verify the exact config
 * shape against the installed tauri-driver version". Verified against
 * tauri-driver 2.0.6's own source (src/cli.rs, src/server.rs): it reads NO
 * configuration file at all, in any version. Its entire config surface is
 *
 *   1. process flags, parsed by pico_args in src/cli.rs:
 *        --port NUMBER         intermediary port      (default 4444)
 *        --native-port NUMBER  underlying WebDriver   (default 4445)
 *        --native-host HOST    underlying WebDriver   (default 127.0.0.1, Linux only)
 *        --native-driver PATH  path to the native WebDriver binary
 *      It is strict: any unrecognised argument is a hard exit.
 *
 *   2. one W3C capability, `tauri:options`, on `capabilities.alwaysMatch`,
 *      deserialized by `struct TauriOptions` (serde `rename_all = "camelCase"`):
 *        application    PathBuf       required
 *        args           Vec<String>   optional
 *        webviewOptions Value         optional, Windows only
 *      `map_capabilities` strips it and substitutes the native object --
 *      `webkitgtk:browserOptions {binary, args}` on Linux,
 *      `ms:edgeOptions` + `browserName: "webview2"` on Windows.
 *
 * So the configuration lives here, in the WebdriverIO config, and nowhere else.
 *
 * PLATFORM SUPPORT. tauri-driver is compiled only for Linux and Windows:
 * src/main.rs gates the real `main` behind
 * `#[cfg(any(target_os = "linux", windows))]` and compiles a stub otherwise
 * that prints "tauri-driver is not supported on this platform" and exits 1.
 * There is no macOS backend (its README lists macOS as "[Todo] ... (probably)"
 * via Appium Mac2). This harness therefore runs on Linux and Windows only --
 * see README.md.
 */

/**
 * The specs that publish releases to the E2E box's GitHub stand-in cannot run
 * against the real github.com, so a native run leaves them out. The box sets
 * QUIVER_E2E_UPSTREAM_STATE (docker/e2e/scenarios/run-wdio.sh).
 */
const BOX_ONLY_SPECS = ['update-core', 'self-update-while-running', 'arrow-apps', 'qol'].map((name) =>
	path.join(import.meta.dirname, 'scenarios', `${name}.spec.ts`)
);

const DRIVER_PORT = Number(process.env.QUIVER_E2E_DRIVER_PORT ?? 4444);
const NATIVE_PORT = Number(process.env.QUIVER_E2E_NATIVE_PORT ?? 4445);

let driver: ChildProcess | null = null;

function portOpen(port: number): Promise<boolean> {
	return new Promise((resolve) => {
		const socket = net.connect({ host: '127.0.0.1', port });
		const done = (ok: boolean) => {
			socket.destroy();
			resolve(ok);
		};
		socket.once('connect', () => done(true));
		socket.once('error', () => done(false));
		setTimeout(() => done(false), 500);
	});
}

async function waitForPort(port: number, timeoutMs = 20_000): Promise<void> {
	const deadline = Date.now() + timeoutMs;
	while (Date.now() < deadline) {
		if (await portOpen(port)) return;
		await new Promise((r) => setTimeout(r, 200));
	}
	throw new Error(`tauri-driver never listened on 127.0.0.1:${port} within ${timeoutMs}ms`);
}

/** `<spec path>` -> `bootstrap`, the per-spec home's name. */
function specName(specs: string[]): string {
	const first = specs[0] ?? 'unknown.spec.ts';
	return path.basename(first.replace(/^file:\/\//, '')).replace(/\.spec\.ts$/, '');
}

export const config: WebdriverIO.Config = {
	runner: 'local',
	tsConfigPath: path.join(import.meta.dirname, 'tsconfig.json'),

	specs: [path.join(import.meta.dirname, 'scenarios', '**', '*.spec.ts')],
	exclude: process.env.QUIVER_E2E_UPSTREAM_STATE ? [] : BOX_ONLY_SPECS,

	// One real GUI window at a time. These scenarios each own a whole
	// QUIVER_HOME and a whole daemon; running two concurrently would have them
	// fighting over the same fixed Windows port and the same driver ports.
	maxInstances: 1,

	capabilities: [
		{
			// No `browserName`, matching Tauri's own documented WebdriverIO
			// example: tauri-driver rewrites `tauri:options` into the native
			// browser object before the request ever reaches WebKitWebDriver or
			// msedgedriver, and naming a browser here would make WDIO try to
			// manage a driver binary of its own.
			'tauri:options': {
				application: APP_BINARY,
			},
		} as WebdriverIO.Capabilities,
	],

	// Talk to tauri-driver, which this config starts itself (below) rather than
	// delegating to a WDIO service -- the process needs a per-spec environment,
	// which is not something a service can express.
	hostname: '127.0.0.1',
	port: DRIVER_PORT,
	path: '/',

	logLevel: (process.env.QUIVER_E2E_LOG_LEVEL as WebdriverIO.Config['logLevel']) ?? 'warn',
	waitforTimeout: 30_000,
	connectionRetryTimeout: 120_000,
	connectionRetryCount: 2,

	framework: 'mocha',
	reporters: ['spec'],
	mochaOpts: {
		ui: 'bdd',
		// A real app launch, a real daemon boot and (scenario 2) a real
		// process handover. Nothing here is fast.
		timeout: 600_000,
	},

	onPrepare() {
		if (!fs.existsSync(APP_BINARY)) {
			throw new Error(
				`the application under test does not exist at ${APP_BINARY}.\n` +
					`Build it first:  bun run tauri build --no-bundle\n` +
					`(or set QUIVER_E2E_APP_BINARY to an existing release binary)`
			);
		}
		fs.mkdirSync(E2E_TMP, { recursive: true });
	},

	/**
	 * Starts one tauri-driver per spec, with that spec's own HOME.
	 *
	 * The environment has to be set HERE and not later: tauri-driver spawns
	 * the native WebDriver, which in turn spawns the application binary, and
	 * each inherits its parent's environment. By the time a session exists the
	 * app process is already running with whatever HOME it was given.
	 */
	async beforeSession(_config, _capabilities, specs) {
		const name = specName(specs);
		const home = homeForSpec(name);

		// A genuinely clean QUIVER_HOME for every spec -- scenario 1's whole
		// precondition, and cheap insurance for the other two.
		fs.rmSync(home, { recursive: true, force: true });
		fs.mkdirSync(quiverHome(home), { recursive: true });

		// The version check is throttled to one remote call per arrow per hour,
		// which no scenario that publishes a release a minute in can live with.
		if (process.env.QUIVER_E2E_VERSION_CHECK_TTL) {
			fs.writeFileSync(
				path.join(quiverHome(home), 'config.yaml'),
				`config:\n  arrows:\n    version_check_ttl: ${process.env.QUIVER_E2E_VERSION_CHECK_TTL}\n`
			);
		}

		// The "no self-installed Core yet" precondition has to be recorded
		// HERE and not inside a spec. By the time Mocha runs its first `it`,
		// tauri-driver has already launched the app, the app has already
		// spawned the daemon, and the daemon has already self-installed -- so a
		// spec checking the path directly would (correctly) find a binary and
		// fail, while proving nothing about the state it started from.
		process.env.QUIVER_E2E_CLEAN_HOME = String(!fs.existsSync(selfInstalledCore(home)));

		// Read back by the specs themselves (same worker process).
		process.env.QUIVER_E2E_HOME = home;
		process.env.QUIVER_E2E_SPEC = name;

		driver = spawn(
			process.env.QUIVER_E2E_TAURI_DRIVER ?? 'tauri-driver',
			[`--port=${DRIVER_PORT}`, `--native-port=${NATIVE_PORT}`],
			{
				stdio: ['ignore', 'inherit', 'inherit'],
				env: {
					...process.env,
					HOME: home,
					// Core reads its own home the same way the app does, and the
					// app only agrees with it while QUIVER_HOME is the platform
					// default beneath HOME. See lib/paths.ts `homeForSpec`.
					QUIVER_HOME: quiverHome(home),
				},
			}
		);
		driver.on('error', (err) => {
			throw new Error(`failed to start tauri-driver: ${err.message}`);
		});

		await waitForPort(DRIVER_PORT);
	},

	/** A failing test leaves the screen it failed on. */
	async afterTest(test, _context, { passed }) {
		const dir = process.env.QUIVER_E2E_RESULTS;
		if (passed || !dir) return;
		const folder = path.join(dir, process.env.QUIVER_E2E_SPEC ?? 'spec');
		fs.mkdirSync(folder, { recursive: true });
		const name = test.title.replace(/[^a-z0-9]+/gi, '-').slice(0, 80);
		try {
			await browser.saveScreenshot(path.join(folder, `FAILED-${name}.png`));
		} catch {
			/* the session may already be gone */
		}
	},

	afterSession() {
		driver?.kill();
		driver = null;
	},
};
