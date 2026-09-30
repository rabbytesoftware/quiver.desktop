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

/**
 * A daemon whose catalog lists quiver.desktop under `selves` (plus an
 * unrelated arrow), whose repository publishes `channels` (the first is the
 * default a refless add picks), and which fails the calls in `failing`
 * (`METHOD path`).
 */
function daemon({
	selves = [] as string[],
	channels = ['nightly-rolling'] as string[],
	failing = {} as Record<string, unknown>,
} = {}): void {
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
					versions: selves.map((ref) => ({ ref, state: 'absent' })),
				},
				{
					namespace: 'github.com/char2cs/crowbar',
					name: 'crowbar',
					description: '',
					tags: [],
					versions: [{ ref: 'stable-26.5', state: 'ready' }],
				},
			]);
		}
		return Promise.resolve(undefined);
	});
	mockApiRequest.mockImplementation((path: string, init?: RequestInit) => {
		const call = `${init?.method ?? 'GET'} ${path}`;
		return fail(call) ?? Promise.resolve({ status: 201, data: undefined });
	});
}

/** Every call but the catalog and channel listings, in the order it was made. */
function calls(): string[] {
	const all = [
		...mockApiRequest.mock.calls.map((args, i) => [mockApiRequest.mock.invocationCallOrder[i], args] as const),
		...mockApiFetch.mock.calls.map((args, i) => [mockApiFetch.mock.invocationCallOrder[i], args] as const),
	];
	return all
		.sort(([a], [b]) => a - b)
		.map(([, [path, init]]) => `${(init as RequestInit | undefined)?.method ?? 'GET'} ${path}`)
		.filter((call) => call !== `GET ${LIST}` && !call.endsWith('/channels'));
}

function deletes(): string[] {
	return calls().filter((call) => call.startsWith('DELETE'));
}

function adoptBody(): unknown {
	const call = mockApiFetch.mock.calls.find(([path]) => String(path).endsWith('/adopt'));
	return call ? JSON.parse(String((call[1] as RequestInit).body)) : undefined;
}

const STABLE = `${NS}@stable`;

beforeEach(() => {
	vi.clearAllMocks();
	builtFrom(null);
	daemon();
});

describe('announceSelf, from a build cut from a release tag', () => {
	it('registers the stable channel, then adopts the tag it was built from as what is installed', async () => {
		builtFrom('stable-26.5.0');

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}`, `POST /v0/arrow/${enc(STABLE)}/adopt`]);
		expect(adoptBody()).toEqual({ resolved_ref: 'stable-26.5.0' });
	});

	it('sends the register with no body at all', async () => {
		builtFrom('stable-26.5.0');
		await announceSelf();
		expect(mockApiRequest).toHaveBeenCalledWith(`/v0/arrow/${enc(STABLE)}`, { method: 'POST' });
	});

	it('does not register again an identity already in the library -- a re-announce only adopts, which core makes a no-op', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: ['stable'] });

		await announceSelf();

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}/adopt`]);
	});

	it('advances the same identity to a newer build through adopt, never a new row', async () => {
		builtFrom('stable-26.9.0');
		daemon({ selves: ['stable'] });

		await announceSelf();

		expect(adoptBody()).toEqual({ resolved_ref: 'stable-26.9.0' });
		expect(deletes()).toEqual([]);
	});

	it.each([400, 404])(
		'leaves the plain registration and logs at debug when core refuses the tag (%i)',
		async (status) => {
			builtFrom('stable-26.5.0');
			daemon({ failing: { [`POST /v0/arrow/${enc(STABLE)}/adopt`]: new ApiError('not a member', status) } });
			const debug = vi.spyOn(console, 'debug').mockImplementation(() => {});
			const error = vi.spyOn(console, 'error').mockImplementation(() => {});

			await expect(announceSelf()).resolves.toBeUndefined();

			expect(debug).toHaveBeenCalled();
			expect(error).not.toHaveBeenCalled();
			debug.mockRestore();
			error.mockRestore();
		}
	);

	it('logs any other adopt failure as an error, and still resolves', async () => {
		builtFrom('stable-26.5.0');
		daemon({ failing: { [`POST /v0/arrow/${enc(STABLE)}/adopt`]: new ApiError('fetch failed', 502) } });
		const error = vi.spyOn(console, 'error').mockImplementation(() => {});

		await expect(announceSelf()).resolves.toBeUndefined();

		expect(error).toHaveBeenCalled();
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

	it('still announces -- refless -- when the build tag cannot be read at all', async () => {
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

describe('announceSelf, removing the rows earlier builds left behind', () => {
	it('removes nothing when this build’s identity is the only one', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: ['stable'] });
		await announceSelf();
		expect(deletes()).toEqual([]);
	});

	it('removes every other quiver.desktop identity, and never the current one or another arrow', async () => {
		builtFrom('stable-26.5.0');
		daemon({ selves: ['stable-26.4', 'stable', 'stable-26.5'] });

		await announceSelf();

		expect(deletes()).toEqual([
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.5`)}`,
		]);
	});

	it('keeps the default-channel row of an untagged build, across every restart, and forgets the rest', async () => {
		daemon({ selves: ['stable-26.4', 'nightly-rolling'] });

		await announceSelf();
		await announceSelf();

		expect(deletes()).toEqual([
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
			`DELETE /v0/arrow/${enc(`${NS}@stable-26.4`)}`,
		]);
		expect(deletes()).not.toContain(`DELETE /v0/arrow/${enc(`${NS}@nightly-rolling`)}`);
	});

	it('logs a failed removal and carries on with the rest', async () => {
		builtFrom('stable-26.5.0');
		daemon({
			selves: ['stable-26.4', 'stable', 'stable-26.5'],
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

		expect(calls()).toEqual([`POST /v0/arrow/${enc(STABLE)}`, `POST /v0/arrow/${enc(STABLE)}/adopt`]);
		expect(error).toHaveBeenCalled();
		error.mockRestore();
	});
});
