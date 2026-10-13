import { describe, expect, it } from 'vitest';

import adoptNewest from './__fixtures__/live-core/adopt-detail-newest-adopted.json';
import adoptOlder from './__fixtures__/live-core/adopt-detail-older-adopted.json';
import adoptList from './__fixtures__/live-core/adopt-list-after-readopt.json';
import channelsCore from './__fixtures__/live-core/channels-core.json';
import channelsCrowbar from './__fixtures__/live-core/channels-crowbar.json';
import detailCoreDevelopInstalled from './__fixtures__/live-core/detail-core-develop-installed.json';
import detailCoreStableOutdated from './__fixtures__/live-core/detail-core-stable-outdated.json';
import detailCrowbarNightly from './__fixtures__/live-core/detail-crowbar-nightly.json';
import detailUncatalogued from './__fixtures__/live-core/detail-uncatalogued-default-channel.json';
import list from './__fixtures__/live-core/list.json';
import manifestCrowbarNightly from './__fixtures__/live-core/manifest-crowbar-nightly.json';
import type { ArrowDetailDTO, ArrowListResponseItemDTO, ArrowManifestDTO, ChannelListDTO } from './arrow';
import { toArrowCatalogRecords, toArrowChannels, toArrowDetail } from './arrow';

// Payloads captured verbatim from a quiver.core built from the request-keyed
// versioning branch, so the mappers are held to the real wire, not to a
// hand-written picture of it.
const manifest = manifestCrowbarNightly.data as unknown as ArrowManifestDTO;

function detailOf(envelope: { data: unknown }): ArrowDetailDTO {
	return envelope.data as ArrowDetailDTO;
}

describe('mapping real quiver.core payloads', () => {
	it('maps a channel row registered as crowbar@nightly', () => {
		const detail = toArrowDetail(detailOf(detailCrowbarNightly), manifest, [], null, [], []);
		expect(detail.namespace).toBe('github.com/char2cs/crowbar@nightly');
		expect(detail.selector).toBe('nightly');
		expect(detail.selector_kind).toBe('channel');
		expect(detail.resolved_ref).toBe('nightly');
		expect(detail.installed_commit).toBe('9dd0b183177a64ec71a2672d1cd7cf0c70bb4877');
		expect(detail.available).toBeNull();
		expect(detail.outdated).toBe(false);
		expect(detail.license).toBe('AGPL-3.0-only OR LicenseRef-Commercial');
	});

	it('reads the author metadata from the nested manifest', () => {
		const detail = toArrowDetail(detailOf(detailCrowbarNightly), manifest, [], null, [], []);
		expect(detail.url).toBe('https://github.com/char2cs/crowbar');
		expect(detail.maintainers[0].name).toBe('char2cs');
		expect(detail.media.icon).toContain('crowbar-icon.png');
		expect(detail.netbridge).toEqual([]);
		expect(detail.targets.map((t) => t.platform)).toContain('darwin/arm64');
	});

	it('maps an outdated channel row with what is available ahead of it', () => {
		const detail = toArrowDetail(detailOf(detailCoreStableOutdated), manifest, [], null, [], []);
		expect(detail.namespace).toBe('github.com/rabbytesoftware/quiver.core@stable');
		expect(detail.state).toBe('outdated');
		expect(detail.resolved_ref).toBe('live');
		expect(detail.available).toEqual({
			ref: 'stable-26.5.1',
			commit: 'e870cda0a7c16a896fb4736a9b34bab1879a057e',
		});
		expect(detail.outdated).toBe(true);
		expect(detail.tags).toEqual([]);
	});

	it('maps an installed pin that is current', () => {
		const detail = toArrowDetail(detailOf(detailCoreDevelopInstalled), manifest, [], null, [], []);
		expect(detail.selector_kind).toBe('pin');
		expect(detail.installed_at).toBe('2026-09-30T00:12:17Z');
		expect(detail.available).toBeNull();
		expect(detail.last_return?.outcome).toBe('success');
	});

	it('maps an uncatalogued refless read, which core files under the default channel', () => {
		const detail = toArrowDetail(detailOf(detailUncatalogued), manifest, [], null, [], []);
		expect(detail.namespace).toBe('github.com/rabbytesoftware/quiver.desktop@nightly-rolling');
		expect(detail.user_installed).toBe(false);
		expect(detail.selector_kind).toBe('channel');
	});

	it('maps the list: one record per identity, versioned by the resolved ref', () => {
		const records = toArrowCatalogRecords(list.data as ArrowListResponseItemDTO[], 'local');
		expect(records.map((r) => [r.namespace, r.version])).toEqual([
			['github.com/char2cs/crowbar@develop', 'develop'],
			['github.com/char2cs/crowbar@nightly', 'nightly'],
			['github.com/rabbytesoftware/quiver.core@develop', 'develop'],
			['github.com/rabbytesoftware/quiver.core@stable', 'live'],
		]);
		expect(records.find((r) => r.namespace.endsWith('quiver.core@stable'))?.tags).toEqual([]);
	});

	it('maps both channel kinds', () => {
		expect(toArrowChannels(channelsCrowbar.data as ChannelListDTO)).toEqual([
			{ name: 'nightly', kind: 'pointer', latest: 'nightly' },
		]);
		const core = toArrowChannels(channelsCore.data as ChannelListDTO);
		expect(core.map((c) => [c.name, c.kind])).toEqual([
			['stable', 'ordered'],
			['beta', 'ordered'],
			['beta-2026-09-27', 'pointer'],
			['nightly-latest', 'pointer'],
		]);
		expect(core[0].members?.[0]).toBe(core[0].latest);
	});

	// A channel identity adopted at a release older than the channel's newest
	// (`POST /adopt`, then a re-check), captured from a core serving a local
	// repository tagged stable-26.5.0 / stable-26.5.1 / stable-26.6.0.
	it('maps an adopted older build: resolved_ref is the build, available the newer release', () => {
		const detail = toArrowDetail(detailOf(adoptOlder), manifest, [], null, [], []);
		expect(detail.namespace).toBe('localhost/tester/desk@stable');
		expect(detail.selector_kind).toBe('channel');
		expect(detail.resolved_ref).toBe('stable-26.5.0');
		expect(detail.available?.ref).toBe('stable-26.6.0');
		expect(detail.outdated).toBe(true);
		expect(detail.state).toBe('outdated');
	});

	it('maps an adopted newest build with nothing available', () => {
		const detail = toArrowDetail(detailOf(adoptNewest), manifest, [], null, [], []);
		expect(detail.resolved_ref).toBe('stable-26.6.0');
		expect(detail.available).toBeNull();
		expect(detail.outdated).toBe(false);
	});

	it('keeps one row after the same build is adopted again', () => {
		const records = toArrowCatalogRecords(adoptList.data as ArrowListResponseItemDTO[], 'local');
		expect(records.filter((r) => r.namespace.startsWith('localhost/tester/desk@'))).toEqual([
			expect.objectContaining({ namespace: 'localhost/tester/desk@stable', version: 'stable-26.5.0' }),
		]);
	});
});
