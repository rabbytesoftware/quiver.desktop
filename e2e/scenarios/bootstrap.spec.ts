import fs from 'node:fs';

import { expect } from '@wdio/globals';

import { waitForAppReady, waitForSidebarArrow, sidebarArrowNames } from '../lib/app-ready';
import { getArrow, waitForCore } from '../lib/core-api';
import { NS_CORE, NS_DESKTOP, quiverHome, selfInstalledCore } from '../lib/paths';

/**
 * SCENARIO 1 -- bootstrap from a genuinely empty QUIVER_HOME.
 *
 * `beforeSession` (wdio.conf.ts) deletes this spec's home before the app is
 * launched, so nothing below is inherited from a previous run: no catalog, no
 * event store, and above all no already-self-installed Core.
 *
 * What this proves that the per-repo unit and integration tests cannot: the
 * bootstrap really happens in the shipped artifact. quiver.core's own Go tests
 * drive `selfarrow.PromoteRunningBinary` and `EnsureRegistered` directly;
 * quiver.desktop's Rust tests drive `resolve_binary_path` against a temp dir.
 * Neither ever launches the built app, spawns the real sidecar, or lets the
 * two find each other.
 */
describe('bootstrap: a clean QUIVER_HOME comes up self-installed', () => {
	const home = process.env.QUIVER_E2E_HOME!;

	it('started from a home with no self-installed Core', () => {
		// Recorded by `beforeSession` BEFORE the app was launched -- see the
		// comment there. Checking `selfInstalledCore(home)` from inside a spec
		// would be checking it long after the daemon already self-installed.
		expect(process.env.QUIVER_E2E_CLEAN_HOME).toBe('true');
	});

	it('reaches a ready state in the real window', async () => {
		await waitForAppReady();

		// The shell is up; the catalog has settled out of its skeleton. Both
		// are real rendered state, read from the selectors the components
		// actually emit (see lib/app-ready.ts).
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
		// `PromoteRunningBinary` writes with mode 0o755 -- the executable bit
		// is the difference between a copied file and a runnable successor.
		expect(stat.mode & 0o111).toBeGreaterThan(0);
	});

	it('registers quiver.core as its own user-installed arrow', async () => {
		await waitForCore(home);

		const { status, body } = await getArrow(home, NS_CORE);

		expect(status).toBe(200);
		expect(body).not.toBeNull();
		expect(body!.namespace).toContain('quiver.core');
		// The claim that matters: core did not merely resolve its own
		// namespace, it holds a catalog row it considers user-installed.
		// `selfarrow.EnsureRegistered` seeds it from the EMBEDDED ARROW.md
		// (internal/core/selfmanifest), and `Seed` sets UserInstalled = true.
		expect(body!.user_installed).toBe(true);
	});

	it('registers quiver.desktop as its own user-installed arrow', async () => {
		await waitForCore(home);

		const { status, body } = await getArrow(home, NS_DESKTOP);

		// This row exists only because the running app announced itself:
		// `announceSelf` (src/lib/core-store/listeners/self-announce.ts) POSTs
		// /v0/arrow/<ns> on every successful connection, and core's Add-time
		// `preinstalled` check has to agree the app is really here.
		//
		// PRECONDITION, and it is a real one rather than a test artefact:
		// unlike quiver.core's, quiver.desktop's manifest is NOT embedded in
		// core. Add resolves it from the remote through
		// `metadata.GetPlatforms()`, so it requires quiver.desktop's ARROW.md
		// to be reachable on github.com.
		//
		// Which ref it looks under depends on how the app under test was
		// built, and BOTH answers clear that precondition through the normal
		// merge path:
		//
		//   - built from a `stable-*` tag (what .github/workflows/e2e.yml
		//     does on its real trigger, a tag push): the binary carries that
		//     tag, the announce is `<ns>@<tag>`, and core takes the explicit
		//     ref as written -- resolvable, because the tag that triggered
		//     the run is on the remote.
		//   - built from anything else (a local run, a `workflow_dispatch`):
		//     no tag is baked in, so the announce is REFLESS and core's
		//     `resolveRefless` answers with the latest `stable-*` release, or
		//     the default branch (`develop`) if there is none.
		//
		// Running this from a branch that has not merged yet still 404s, and
		// that is the honest result, not a harness fault.
		//
		// What it must never go back to: the announce originally carried
		// `@0.1.0` (tauri.conf.json's productVersion), and core takes an
		// explicit ref as written with no fallback, so it demanded a git ref
		// named `0.1.0` that nothing in this repo's release process -- which
		// tags `stable-<series>[.patch]` -- ever creates. That 404 was
		// permanent, not pending.
		expect(status).toBe(200);
		expect(body).not.toBeNull();
		expect(body!.namespace).toContain('quiver.desktop');
		expect(body!.user_installed).toBe(true);
	});

	it('shows both self-arrows in the sidebar the user actually sees', async () => {
		// Names come from each ARROW.md's `metadata.name`: "Quiver Core" and
		// "Quiver".
		const names = await waitForSidebarArrow('Quiver Core');
		expect(names).toContain('Quiver Core');

		const all = await sidebarArrowNames();
		expect(all).toContain('Quiver');
	});
});
