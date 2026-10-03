import { execFileSync } from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

/**
 * The container's stand-in for github.com (docker/e2e/fixtures/upstream.py)
 * keeps everything it serves under one state directory. A scenario publishes a
 * new release by writing there, exactly as docker/e2e/scenarios/lib.sh does for
 * the shell scenarios: the daemon under test still reaches it over HTTPS by the
 * real hostname and still verifies what it downloads.
 */
export interface UpstreamEnv {
	state: string;
	/** quiver.core's own ARROW.md, served as the manifest at each published tag. */
	coreManifest: string;
	/** Five quiver.core versions, oldest first. The first four are real builds; the fifth is only published as a program that never becomes healthy. */
	coreTags: [string, string, string, string, string];
	/** A directory holding `<tag>/<asset>` for each built version. */
	coreAssets: string;
	/** The release asset's name, which the daemon's asset picker must recognise as this platform's. */
	coreAsset: string;
}

const CORE_REPO = 'rabbytesoftware/quiver.core';

function required(name: string): string {
	const value = process.env[name];
	if (!value) {
		throw new Error(
			`${name} is unset. This scenario publishes releases to the container's GitHub stand-in and so runs only ` +
				`inside the E2E box: docker compose -f docker/e2e/docker-compose.yml run --rm e2e scenarios/run-wdio.sh update-core`
		);
	}
	return value;
}

export function upstreamEnv(): UpstreamEnv {
	const tags = required('QUIVER_E2E_CORE_TAGS').split(/\s+/).filter(Boolean);
	if (tags.length !== 5) throw new Error(`QUIVER_E2E_CORE_TAGS must list five versions, got "${tags.join(' ')}"`);
	return {
		state: required('QUIVER_E2E_UPSTREAM_STATE'),
		coreManifest: required('QUIVER_E2E_CORE_MANIFEST'),
		coreTags: tags as UpstreamEnv['coreTags'],
		coreAssets: required('QUIVER_E2E_CORE_ASSETS'),
		coreAsset: required('QUIVER_E2E_CORE_ASSET'),
	};
}

function bareRepo(env: UpstreamEnv, repo: string): string {
	return path.join(env.state, 'git', `${repo}.git`);
}

/** Tags the repository's HEAD, which is what quiver.core's version check reads through ls-remote. */
function tag(env: UpstreamEnv, repo: string, name: string): void {
	execFileSync('git', ['--git-dir', bareRepo(env, repo), 'tag', '-f', name, 'HEAD'], { stdio: 'pipe' });
}

/** The release asset a built version was packaged as. */
export function builtAsset(env: UpstreamEnv, tagName: string): string {
	return path.join(env.coreAssets, tagName, env.coreAsset);
}

export interface ReleaseOptions {
	/** Publish this built version's binary under the tag, for a tag that was never built itself. */
	binaryOf?: string;
	/** List a checksum that is not the asset's, as a corrupted or tampered download would. */
	wrongChecksum?: boolean;
	/**
	 * Publish a program that hands over to the real build's `self-update` but is
	 * not a daemon itself, so the swap starts something that never becomes healthy.
	 */
	unhealthy?: boolean;
}

function sha256(file: string): string {
	return crypto.createHash('sha256').update(fs.readFileSync(file)).digest('hex');
}

/**
 * Publishes quiver.core `tagName` upstream: the tag, the manifest at it, the
 * binary and its `checksums.txt` as release assets, and the "latest release"
 * pointer. Returns the published asset's path so a scenario can compare bytes.
 */
export function publishCoreRelease(env: UpstreamEnv, tagName: string, options: ReleaseOptions = {}): string {
	const rawDir = path.join(env.state, 'raw', CORE_REPO, tagName);
	fs.mkdirSync(rawDir, { recursive: true });
	fs.copyFileSync(env.coreManifest, path.join(rawDir, 'ARROW.md'));

	const releaseDir = path.join(env.state, 'releases', CORE_REPO, tagName);
	fs.rmSync(releaseDir, { recursive: true, force: true });
	fs.mkdirSync(releaseDir, { recursive: true });
	const asset = path.join(releaseDir, env.coreAsset);
	const built = builtAsset(env, options.binaryOf ?? tagName);
	if (options.unhealthy) {
		fs.writeFileSync(
			asset,
			`#!/bin/sh\nif [ "$1" = self-update ]; then exec ${JSON.stringify(built)} self-update "$2"; fi\nexec sleep 600\n`
		);
	} else {
		fs.copyFileSync(built, asset);
	}
	fs.chmodSync(asset, 0o755);

	const sum = options.wrongChecksum ? '0'.repeat(64) : sha256(asset);
	fs.writeFileSync(path.join(releaseDir, 'checksums.txt'), `${sum}  ./${env.coreAsset}\n`);

	fs.writeFileSync(path.join(env.state, 'releases', CORE_REPO, 'LATEST'), `${tagName}\n`);
	tag(env, CORE_REPO, tagName);
	return asset;
}

/**
 * Publishes a new tag of a small fixture repository, serving the manifest it
 * already serves at `develop` under the tag too, so a row that follows the
 * repository's `stable` channel sees a newer release ahead of what it has.
 */
export function publishRepoTag(env: UpstreamEnv, repo: string, tagName: string): void {
	const from = path.join(env.state, 'raw', repo, 'develop', 'ARROW.md');
	const to = path.join(env.state, 'raw', repo, tagName);
	fs.mkdirSync(to, { recursive: true });
	fs.copyFileSync(from, path.join(to, 'ARROW.md'));
	tag(env, repo, tagName);
}

/** How many times the stand-in has been asked for `requestPath` (the request log is one JSON object per line). */
export function upstreamHits(requestPath: string): number {
	const log = process.env.QUIVER_E2E_UPSTREAM_LOG;
	if (!log || !fs.existsSync(log)) return 0;
	let hits = 0;
	for (const line of fs.readFileSync(log, 'utf8').split('\n')) {
		if (!line) continue;
		try {
			if ((JSON.parse(line) as { path?: string }).path === requestPath) hits++;
		} catch {
			/* a partial last line while the fixture is still writing */
		}
	}
	return hits;
}
