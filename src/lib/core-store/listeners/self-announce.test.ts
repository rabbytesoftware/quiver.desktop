import { beforeEach, describe, expect, it, vi, type MockedFunction } from 'vitest';

vi.mock('@/lib/transport/api', async (importOriginal) => {
	const actual = await importOriginal<typeof import('@/lib/transport/api')>();
	return { ...actual, apiFetch: vi.fn(), apiRequest: vi.fn() };
});

vi.mock('@/lib/transport/backend', () => ({
	backend: vi.fn(),
}));

import { apiFetch, apiRequest, ApiError } from '@/lib/transport/api';
import type { Backend } from '@/lib/transport/backend';
import { backend } from '@/lib/transport/backend';

import { announceSelf } from './self-announce';
import { useArrowStore } from '../store/arrows';

const mockApiFetch = apiFetch as MockedFunction<typeof apiFetch>;
const mockApiRequest = apiRequest as MockedFunction<typeof apiRequest>;
const mockBackend = backend as MockedFunction<typeof backend>;

const NS = 'github.com/rabbytesoftware/quiver.desktop';
const enc = encodeURIComponent;
const LIST = '/v0/arrow?user_installed=true';

/** Stands in for a binary that was (or was not) built from a release tag. */
function builtFrom(tag: string | null): void {
	mockBackend.mockReturnValue({ getBuildTag: vi.fn().mockResolvedValue(tag) } as unknown as Backend);
}

interface Self {
	ref: string;
	/** What the row resolved to; defaults to the ref itself. */
	resolved?: string;
	/** Defaults to `channel` for `stable`/`nightly-rolling`, `pin` for anything else -- what earlier builds created. `null` leaves the field out. */
	kind?: string | null;
}

const CHANNEL_REFS = ['stable', 'nightly-rolling'];

/**
 * A daemon whose catalog lists quiver.desktop under `selves` (plus an
 * unrelated arrow), whose repository publishes `channels` (the first is the
 * default a refless add picks), and which fails the calls in `failing`
 * (`METHOD path`).
 */
function daemon({
	selves = [] as (string | Self)[],
	channels = ['nightly-rolling'] as string[],
	failing = {} as Record<string, unknown>,
} = {}): void {
	const rows: Self[] = selves.map((self) => (typeof self === 'string' ? { ref: self } : self));
	const kindOf = (row: Self) => row.kind ?? (CHANNEL_REFS.includes(row.ref) ? 'channel' : 'pin');
	const fail = (call: string) => (call in failing ? Promise.reject(failing[call]) : undefined);
	mockApiFetch.mockImplementation((path: string, init?: RequestInit) => {
		const call = `${init?.method ?? 'GET'} ${path}`;
		const failure = fail(call);
		if (failure) return failure;
		if (call === `GET /v0/arrow/${enc(NS)}/channels`) {
			return Promise.resolve({ channels: channels.map((name) => ({ name, kind: 'pointer', latest: name })) });
		}
		if (call === `GET ${LIST}`) {
			return Promise.resolve([
				{
					namespace: NS,
					name: 'Quiver',
					description: '',
					tags: null,
					versions: rows.map((row) => ({
						ref: row.ref,
						resolved_ref: row.resolved ?? row.ref,
						state: 'absent',
					})),
				},
				{
					namespace: 'github.com/char2cs/crowbar',
					name: 'crowbar',
					description: '',
					tags: [],
					versions: [{ ref: 'stable-26.5', resolved_ref: 'stable-26.5', state: 'ready' }],
				},
			]);
		}
		const row = rows.find((r) => call === `GET /v0/arrow/${enc(`${NS}@${r.ref}`)}`);
		if (row) {
			return Promise.resolve({
				namespace: `${NS}@${row.ref}`,
				...(row.kind === null ? {} : { selector_kind: kindOf(row) }),
				resolved_ref: row.resolved ?? row.ref,
			});
		}
		return Promise.resolve(undefined);
	});
	mockApiRequest.mockImplementation((path: string, init?: RequestInit) => {
		const call = `${init?.method ?? 'GET'} ${path}`;
		return fail(call) ?? Promise.resolve({ status: 201, data: undefined });
	});
}

/** Every call, in the order it was made. */
function allCalls(): string[] {
	const all = [
		...mockApiRequest.mock.calls.map((args, i) => [mockApiRequest.mock.invocationCallOrder[i], args] as const),
		...mockApiFetch.mock.calls.map((args, i) => [mockApiFetch.mock.invocationCallOrder[i], args] as const),
	];
	return all
		.sort(([a], [b]) => a - b)
		.map(([, [path, init]]) => `${(init as RequestInit | undefined)?.method ?? 'GET'} ${path}`);
}

/** Every mutation, in the order it was made. */
function calls(): string[] {
	return allCalls().filter((call) => !call.startsWith('GET'));
}

function deletes(): string[] {
	return calls().filter((call) => call.startsWith('DELETE'));
}

function adoptBody(): unknown {
	const call = mockApiFetch.mock.calls.find(([path]) => String(path).endsWith('/adopt'));
	return call ? JSON.parse(String((call[1] as RequestInit).body)) : undefined;
}

const STABLE = `${NS}@stable`;
const ADOPT = `POST /v0/arrow/${enc(STABLE)}/adopt`;

beforeEach(() => {
	vi.clearAllMocks();
	builtFrom(null);
	daemon();
	useArrowStore.getState().setCatalogRefresh(() => {});
});

describe('announceSelf, from a build cut from a release tag', () => {
	it('registers the stable channel, then adopts the tag it was built from as what is installed', async () => {
		builtFrom('stable-26.5.0');

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}`, ADOPT]);
		expect(adoptBody()).toEqual({ resolved_ref: 'stable-26.5.0' });
	});

	it('sends the register with no body at all', async () => {
		builtFrom('stable-26.5.0');
		await announceSelf();
		expect(mockApiRequest).toHaveBeenCalledWith(`/v0/arrow/${enc(STABLE)}`, { method: 'POST' });
	});

	it('re-reads the catalog after an adopt, so the sidebar shows the version it declared', async () => {
		builtFrom('stable-26.5.0');
		const refresh = vi.fn();
		useArrowStore.getState().setCatalogRefresh(refresh);
		await announceSelf();
		expect(refresh).toHaveBeenCalledTimes(1);
	});

	it('sends nothing but the listing when the same build announces again', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: [{ ref: 'stable', resolved: 'stable-26.5.0' }] });

		await announceSelf();

		expect(allCalls()).toEqual([`GET ${LIST}`]);
	});

	it('advances the same identity to a newer build through adopt, never a new row', async () => {
		builtFrom('stable-26.9.0');
		daemon({ selves: [{ ref: 'stable', resolved: 'stable-26.5.0' }] });

		await announceSelf();

		expect(calls()).toEqual([ADOPT]);
		expect(adoptBody()).toEqual({ resolved_ref: 'stable-26.9.0' });
	});

	it.each([400, 404])(
		'leaves the plain registration, logs at debug and forgets nothing when core refuses the tag (%i)',
		async (status) => {
			builtFrom('stable-26.5.0');
			daemon({ selves: ['stable-26.4'], failing: { [ADOPT]: new ApiError('not a member', status) } });
			const debug = vi.spyOn(console, 'debug').mockImplementation(() => {});
			const error = vi.spyOn(console, 'error').mockImplementation(() => {});

			await expect(announceSelf()).resolves.toBeUndefined();

			expect(debug).toHaveBeenCalled();
			expect(error).not.toHaveBeenCalled();
			expect(deletes()).toEqual([]);
			debug.mockRestore();
			error.mockRestore();
		}
	);

	it('logs any other adopt failure as an error, forgets nothing, and still resolves', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: ['stable-26.4'], failing: { [ADOPT]: new ApiError('fetch failed', 502) } });
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(error).toHaveBeenCalled();
		expect(deletes()).toEqual([]);
		error.mockRestore();
	});
});

describe('announceSelf, from a build with no release tag', () => {
	const DEFAULT = `${NS}@nightly-rolling`;

	it('registers the repository’s default channel explicitly -- what a refless add would pick -- with no body, and adopts nothing', async () => {
		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(DEFAULT)}`]);
		expect(mockApiRequest).toHaveBeenCalledWith(`/v0/arrow/${enc(DEFAULT)}`, { method: 'POST' });
	});

	it('treats an empty tag as no tag', async () => {
		builtFrom('');
		await announceSelf();
		expect(calls()).toEqual([`POST /v0/arrow/${enc(DEFAULT)}`]);
	});

	it('does not register again when the default channel is already in the library', async () => {
		daemon({ selves: ['nightly-rolling'] });
		await announceSelf();
		expect(calls()).toEqual([]);
	});

	it('falls back to the untagged path -- the default channel by name -- when the build tag cannot be read at all', async () => {
		mockBackend.mockReturnValue({
			getBuildTag: vi.fn().mockRejectedValue(new Error('no ipc')),
		} as unknown as Backend);
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(DEFAULT)}`]);
		expect(error).toHaveBeenCalled();
		error.mockRestore();
	});

	it('registers refless, and forgets nothing, when the channels cannot be read', async () => {
		daemon({ selves: ['stable-26.4'], failing: { [`GET /v0/arrow/${enc(NS)}/channels`]: new Error('down') } });
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(NS)}`]);
		error.mockRestore();
	});

	it('registers refless, and forgets nothing, when the repository publishes no channel', async () => {
		daemon({ selves: ['stable-26.4'], channels: [] });
		await announceSelf();
		expect(calls()).toEqual([`POST /v0/arrow/${enc(NS)}`]);
	});
});

describe('announceSelf, removing the pin rows earlier builds left behind', () => {
	it('removes nothing when this build’s identity is the only one', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: [{ ref: 'stable', resolved: 'stable-26.5.0' }] });
		await announceSelf();
		expect(deletes()).toEqual([]);
	});

	it('removes every other quiver.desktop pin, and never the current one or another arrow', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: ['stable-26.4', { ref: 'stable', resolved: 'stable-26.5.0' }, 'stable-26.5'] });

		await announceSelf();

		expect(deletes()).toEqual([
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.5`)}`,
		]);
	});

	it('never removes a sibling channel identity -- another build’s own row', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: [{ ref: 'stable', resolved: 'stable-26.5.0' }, 'nightly-rolling', 'stable-26.4'] });

		await announceSelf();

		expect(deletes()).toEqual([`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`]);
	});

	it('keeps an untagged build’s default-channel row, and every channel row, across restarts', async () => {
		daemon({ selves: ['stable-26.4', 'nightly-rolling', 'stable'] });

		await announceSelf();
		await announceSelf();

		expect(deletes()).toEqual([
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
		]);
	});

	it('keeps a row whose kind cannot be read', async () => {
		builtFrom('stable-26.5.0');
		daemon({
			selves: [{ ref: 'stable', resolved: 'stable-26.5.0' }, 'stable-26.4'],
			failing: { [`GET /v0/arrow/${enc(`${NS}@stable-26.4`)}`]: new Error('down') },
		});
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await announceSelf();

		expect(deletes()).toEqual([]);
		error.mockRestore();
	});

	it('keeps a row whose detail names no selector kind', async () => {
		builtFrom('stable-26.5.0');
		daemon({
			selves: [
				{ ref: 'stable', resolved: 'stable-26.5.0' },
				{ ref: 'stable-26.4', kind: null },
			],
		});

		await announceSelf();

		expect(deletes()).toEqual([]);
	});

	it('logs a failed removal and carries on with the rest', async () => {
		builtFrom('stable-26.5.0');
		daemon({
			selves: ['stable-26.4', { ref: 'stable', resolved: 'stable-26.5.0' }, 'stable-26.5'],
			failing: {
				[`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`]: new Error('other arrows depend on this arrow'),
			},
		});
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(deletes()).toContain(`DELETE /v0/arrow/${enc(`${NS}@stable-26.5`)}`);
		expect(error).toHaveBeenCalled();
		error.mockRestore();
	});

	it('removes nothing, and adopts nothing, when the register itself fails', async () => {
		builtFrom('stable-26.5.0');
		daemon({
			selves: ['stable-26.4'],
			failing: { [`POST /v0/arrow/${enc(STABLE)}`]: new Error('502 bad gateway') },
		});
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}`]);
		expect(error).toHaveBeenCalled();
		error.mockRestore();
	});

	it('still registers and adopts when the catalog cannot be listed, and removes nothing', async () => {
		builtFrom('stable-26.5.0');
		daemon({ failing: { [`GET ${LIST}`]: new Error('down') } });
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}`, ADOPT]);
		expect(error).toHaveBeenCalled();
		error.mockRestore();
	});
});
