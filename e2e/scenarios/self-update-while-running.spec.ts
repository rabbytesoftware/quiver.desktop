import { expect } from '@wdio/globals';
import crypto from 'node:crypto';
import http from 'node:http';
import type { AddressInfo } from 'node:net';

import { waitForAppReady } from '../lib/app-ready';
import { coreClient, getArrow, processAlive, waitForCore, type ArrowDetailDTO } from '../lib/core-api';
import { NS_CORE } from '../lib/paths';

// SCENARIO 2 -- a process quiver.core supervises is not killed by quiver.core's
// own self-update, driven through the real Desktop app and a real OS-level
// exec handover (quiver.core's own selfupdate_test.go can only simulate that
// part, since it can't replace its own test binary's process image).
//
// The supervised arrow must be one a real daemon can resolve through
// metadata.yaml's embedded platforms, not the Go test's in-process stub
// resolver. QUIVER_E2E_FIXTURE_NS names it.
const FIXTURE_NS = process.env.QUIVER_E2E_FIXTURE_NS ?? '';

/** Serves one payload and reports the checksum core must verify it against. */
async function serveReleaseAsset(payload: Buffer): Promise<{ url: string; checksum: string; close: () => void }> {
	const server = http.createServer((_req, res) => {
		res.writeHead(200, { 'Content-Type': 'application/octet-stream', 'Content-Length': payload.length });
		res.end(payload);
	});
	await new Promise<void>((resolve) => server.listen(0, '127.0.0.1', resolve));
	const { port } = server.address() as AddressInfo;
	return {
		url: `http://127.0.0.1:${port}/quiver-new`,
		checksum: crypto.createHash('sha256').update(payload).digest('hex'),
		close: () => server.close(),
	};
}

describe('self-update while a supervised process is running', () => {
	const home = process.env.QUIVER_E2E_HOME!;
	let selfNs = '';

	before(async () => {
		await waitForAppReady();
		await waitForCore(home);

		const { body } = await getArrow(home, NS_CORE);
		expect(body).not.toBeNull();
		selfNs = body!.installed_ref ? `${NS_CORE}@${body!.installed_ref}` : body!.namespace;
		expect(selfNs).toContain('@');
	});

	it('has a supervised arrow to protect', async function () {
		if (!FIXTURE_NS) {
			throw new Error(
				'QUIVER_E2E_FIXTURE_NS is unset. This scenario needs an arrow a REAL daemon can ' +
					"resolve and run a long-lived `execute` for. quiver.core's own " +
					'`quiver-test/self-update-fixture` is injected through the Go integration ' +
					"suite's in-process stub resolver (tests/kit/env.go) and is unreachable from " +
					"a production daemon, which resolves only through metadata.yaml's embedded " +
					'`platforms` (github.com / gitlab.com / bitbucket.org, not overridable from ' +
					'config.yaml). Publish the fixture to one of those hosts, or add a local ' +
					'platform override to core, then set QUIVER_E2E_FIXTURE_NS.'
			);
		}

		const client = coreClient(home);
		const added = await client.post(`/v0/arrow/${encodeURIComponent(FIXTURE_NS)}`);
		expect([200, 201]).toContain(added.status);
	});

	it('installs and starts the supervised arrow', async () => {
		const client = coreClient(home);

		const install = await client.post(`/v0/runtime/${encodeURIComponent(FIXTURE_NS)}/install`, {});
		expect(install.status).toBe(202);
		await waitForState(home, FIXTURE_NS, 'ready');

		const exec = await client.post(`/v0/runtime/${encodeURIComponent(FIXTURE_NS)}/execute`, {});
		expect(exec.status).toBe(202);
		await waitForState(home, FIXTURE_NS, 'running');
	});

	it('keeps the supervised process alive across the self-update handover', async () => {
		const before = await waitForActivePid(home, FIXTURE_NS);
		expect(before).toBeGreaterThan(0);

		const asset = await serveReleaseAsset(Buffer.from('e2e quiver.core successor binary'));
		try {
			const { status } = await coreClient(home).post(`/v0/runtime/${encodeURIComponent(selfNs)}/update`, {
				variables: {
					QUIVER_RELEASE_ASSET_URL: asset.url,
					QUIVER_RELEASE_CHECKSUM: asset.checksum,
				},
			});
			expect(status).toBe(202);

			await new Promise((r) => setTimeout(r, 2_000));
			await waitForCore(home, 120_000);

			// RecordDetached clears Execution, so there's no active_run.pid
			// left in the read model once Detached -- check the OS process.
			expect(processAlive(before)).toBe(true);

			const after = await getArrow(home, FIXTURE_NS);
			expect(after.status).toBe(200);
			expect(after.body!.state).toBe('detached');
		} finally {
			asset.close();
			try {
				process.kill(before, 'SIGKILL');
			} catch {
				/* already gone */
			}
		}
	});

	it('leaves the Desktop window still usable afterwards', async () => {
		await waitForAppReady();
		await expect($('[data-slot="sidebar"]')).toBeExisting();
	});
});

async function waitForState(home: string, ns: string, state: string, timeout = 180_000): Promise<void> {
	const deadline = Date.now() + timeout;
	let last = '';
	while (Date.now() < deadline) {
		const { body } = await getArrow(home, ns);
		last = body?.state ?? '(no body)';
		if (last === state) return;
		await new Promise((r) => setTimeout(r, 500));
	}
	throw new Error(`${ns} never reached state "${state}" (last seen: "${last}")`);
}

async function waitForActivePid(home: string, ns: string, timeout = 120_000): Promise<number> {
	const deadline = Date.now() + timeout;
	let detail: ArrowDetailDTO | null = null;
	while (Date.now() < deadline) {
		detail = (await getArrow(home, ns)).body;
		const pid = detail?.active_run?.pid;
		if (typeof pid === 'number' && pid > 0) return pid;
		await new Promise((r) => setTimeout(r, 500));
	}
	throw new Error(`${ns} never reported an active_run.pid (last state: "${detail?.state}")`);
}
