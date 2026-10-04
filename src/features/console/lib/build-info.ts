/**
 * What the rail's build indicator says about a build, as a pure function.
 *
 * Two builds are shown, `core` (the daemon) and `app` (this desktop). At rest
 * a rolling build (nightly) reports WHEN it was built, because its tag never
 * changes and a timestamp is the only thing that tells two nightlies apart; a
 * release reports its version and name. On hover either one reports the commit.
 * An unstamped build says `dev`.
 */

export type BuildChannel = 'stable' | 'beta' | 'hotfix' | 'nightly' | 'dev';

export interface BuildDescriptor {
	channel: BuildChannel;
	/**
	 * The release version: a tag (`stable-26.5.1`, `beta-26.5-4`) or the bare
	 * form of one (`26.5.1`, `26.5-4`). Only meaningful for stable, beta and hotfix.
	 */
	version: string | null;
	/** Full or short commit; the indicator shows the first {@link SHORT_COMMIT} characters. */
	commit: string | null;
	/** Unix seconds. */
	builtAt: number | null;
}

export const SHORT_COMMIT = 7;

const CHANNEL_PREFIX = /^(stable|beta|hotfix)-/;

/** `nightly-rolling` is what the pipeline calls the rolling channel; the indicator calls it `nightly`. */
export function normaliseChannel(raw: string | null | undefined): BuildChannel | null {
	switch (raw?.trim().toLowerCase()) {
		case 'stable':
			return 'stable';
		case 'beta':
			return 'beta';
		case 'hotfix':
			return 'hotfix';
		case 'nightly':
		case 'nightly-rolling':
		case 'nightly-latest':
			return 'nightly';
		default:
			return null;
	}
}

/** The channel a release tag belongs to, or null when it is not one. */
export function channelOfLabel(label: string | null | undefined): BuildChannel | null {
	const match = CHANNEL_PREFIX.exec(label?.trim() ?? '');
	return match ? (match[1] as BuildChannel) : null;
}

export interface ParsedVersion {
	/** `26.5.1`, `26.5`, `2026-09-27`. */
	series: string;
	/** The build count of a beta or hotfix tag (`beta-26.5-4` → 4); null when there is none. */
	build: number | null;
}

const SERIES = /^(\d+(?:\.\d+)+|\d{4}-\d{2}-\d{2})(?:-(\d+))?$/;

/** Reads a tag or bare version; null when it does not have the shape of one. */
export function parseVersion(version: string | null | undefined): ParsedVersion | null {
	const bare = (version ?? '').trim().replace(CHANNEL_PREFIX, '');
	const match = SERIES.exec(bare);
	if (!match) return null;
	return { series: match[1], build: match[2] === undefined ? null : Number(match[2]) };
}

function shortCommit(commit: string | null): string | null {
	const trimmed = commit?.trim() ?? '';
	return trimmed ? trimmed.slice(0, SHORT_COMMIT) : null;
}

function pad(n: number): string {
	return String(n).padStart(2, '0');
}

/** `MM-DD HH:mm` in the viewer's time zone. */
export function formatBuildTime(seconds: number, compact = false): string {
	const d = new Date(seconds * 1000);
	const date = `${pad(d.getMonth() + 1)}-${pad(d.getDate())}`;
	return compact ? date : `${date} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export interface FormatOptions {
	/** Show the commit instead of the version or timestamp. */
	hover?: boolean;
	/** Narrow rail: drop the time of day from a timestamp. */
	compact?: boolean;
}

/** The text for one build line, without its `core`/`app` label. */
export function formatBuild(build: BuildDescriptor, options: FormatOptions = {}): string {
	const { channel } = build;
	if (options.hover) {
		const hash = shortCommit(build.commit);
		if (hash) return `${channel} ${hash}`;
		// Nothing to reveal: say the same thing the build says at rest.
	}

	if (channel === 'dev') return 'dev';

	if (channel === 'nightly') {
		return build.builtAt === null ? 'nightly' : `nightly ${formatBuildTime(build.builtAt, options.compact)}`;
	}

	const parsed = parseVersion(build.version);
	if (!parsed) return build.version?.trim() ? `${channel} ${build.version.trim()}` : channel;
	return parsed.build === null ? `${channel} ${parsed.series}` : `${channel} ${parsed.series} #${parsed.build}`;
}

/** What the desktop build knows about itself. */
export interface DesktopStamp {
	/** `VITE_QUIVER_BUILD_CHANNEL`, trimmed; empty for a PR or local build. */
	channel: string;
	/** The release label from `get_build_stamp`. */
	label: string | null;
	commit: string | null;
	builtAt: number | null;
}

/** The desktop's descriptor. A build with neither a known channel nor a release label is `dev`. */
export function describeDesktop(stamp: DesktopStamp): BuildDescriptor {
	const channel = normaliseChannel(stamp.channel) ?? channelOfLabel(stamp.label) ?? 'dev';
	return {
		channel,
		version: stamp.label,
		commit: stamp.commit,
		builtAt: stamp.builtAt,
	};
}

/** The slice of `GET /versions` the indicator reads. */
export interface CoreBuildInfo {
	version: string;
	commit: string;
	builtAt: string;
	channel: string;
}

/** The core's descriptor; a daemon that reports no channel is `dev`. */
export function describeCore(info: CoreBuildInfo | null): BuildDescriptor | null {
	if (info === null) return null;
	const builtAt = Date.parse(info.builtAt);
	return {
		channel: normaliseChannel(info.channel) ?? channelOfLabel(info.version) ?? 'dev',
		version: info.version || null,
		commit: info.commit || null,
		builtAt: Number.isNaN(builtAt) ? null : Math.floor(builtAt / 1000),
	};
}

/** The long form, for a tooltip: everything known about the build. */
export function describeBuildLong(build: BuildDescriptor): string {
	const parts: string[] = [formatBuild({ ...build, builtAt: null, commit: null })];
	if (build.commit) parts.push(build.commit.slice(0, 40));
	if (build.builtAt !== null) parts.push(new Date(build.builtAt * 1000).toISOString().replace('.000Z', 'Z'));
	return parts.join(' · ');
}
