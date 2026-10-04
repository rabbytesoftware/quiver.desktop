import { act, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';

import { ArrowAppHost } from './arrow-app-host';
import { tauriApi } from '../lib/api';
import { SHIM_TAG } from '../lib/bridge';
import { useOpenApps } from '../store/open-apps';

vi.mock('../lib/api', () => ({
	tauriApi: {
		host: vi.fn(async (ns: string) => `h-${ns.replace(/\W/g, '')}`),
		wsOpen: vi.fn(async () => {}),
		wsSend: vi.fn(async () => {}),
		wsClose: vi.fn(async () => {}),
	},
}));

const A = 'a/one@1';
const B = 'b/two@1';

function entry(namespace: string): ArrowEntry {
	return {
		namespace,
		active_run: { method: 'execute', variables: {}, steps: [], surface: { mode: 'listen', path: '/x', ready: true } },
	} as unknown as ArrowEntry;
}

function seed(...namespaces: string[]): void {
	useArrowStore.setState({ arrows: new Map(namespaces.map((n) => [n, entry(n)])) });
}

const frameOf = (ns: string) => screen.getByTitle(`h-${ns.replace(/\W/g, '')}`) as HTMLIFrameElement;

function shimMessage(source: MessageEventSource, data: Record<string, unknown>): void {
	window.dispatchEvent(new MessageEvent('message', { source, data: { [SHIM_TAG]: 1, ...data } }));
}

describe('ArrowAppHost', () => {
	beforeEach(() => {
		useArrowStore.getState().reset();
		useOpenApps.setState({ order: [], visible: null, reloads: {} });
		vi.clearAllMocks();
	});

	async function openBoth() {
		seed(A, B);
		const view = render(<ArrowAppHost />);
		act(() => {
			useOpenApps.getState().show(A);
			useOpenApps.getState().show(B);
			useOpenApps.getState().show(A);
		});
		await waitFor(() => expect(screen.getAllByTitle(/^h-/)).toHaveLength(2));
		return view;
	}

	it('keeps one frame per open app and shows only the visible one', async () => {
		await openBoth();

		expect(frameOf(A).className).not.toContain('hidden');
		expect(frameOf(B).className).toContain('hidden');
		expect(frameOf(A).getAttribute('src')).toBe('arrow-app://h-aone1/x');
		expect(tauriApi.host).toHaveBeenCalledTimes(2);
	});

	it('hiding keeps frames mounted', async () => {
		await openBoth();
		const frame = frameOf(A);
		act(() => useOpenApps.getState().hide());

		expect(frameOf(A)).toBe(frame);
		expect(frame.className).toContain('hidden');
	});

	it('drops a frame and closes its sockets when the arrow loses its surface', async () => {
		await openBoth();
		const win = frameOf(B).contentWindow as Window;
		shimMessage(win, { type: 'ws-open', id: '1', path: '/ws' });
		await waitFor(() => expect(tauriApi.wsOpen).toHaveBeenCalled());

		act(() => seed(A));

		await waitFor(() => expect(screen.getAllByTitle(/^h-/)).toHaveLength(1));
		expect(useOpenApps.getState().order).toEqual([A]);
		expect(tauriApi.wsClose).toHaveBeenCalledWith('h-btwo1', '1');
	});

	it('Reload remounts the frame and closes its sockets', async () => {
		await openBoth();
		const before = frameOf(A);
		shimMessage(before.contentWindow as Window, { type: 'ws-open', id: '7', path: '/ws' });
		await waitFor(() => expect(tauriApi.wsOpen).toHaveBeenCalled());

		act(() => useOpenApps.getState().reload(A));

		await waitFor(() => expect(frameOf(A)).not.toBe(before));
		expect(tauriApi.wsClose).toHaveBeenCalledWith('h-aone1', '7');
	});

	it('relays shim messages from a registered frame only', async () => {
		await openBoth();
		shimMessage(frameOf(A).contentWindow as Window, { type: 'ws-open', id: '1', path: '/ws' });
		await waitFor(() => expect(tauriApi.wsOpen).toHaveBeenCalledWith('h-aone1', '1', '/ws', expect.any(Function)));

		vi.mocked(tauriApi.wsOpen).mockClear();
		shimMessage(window, { type: 'ws-open', id: '2', path: '/ws' });
		await act(async () => {});
		expect(tauriApi.wsOpen).not.toHaveBeenCalled();
	});

	it('stops listening and closes sockets on unmount', async () => {
		const { unmount } = await openBoth();
		shimMessage(frameOf(A).contentWindow as Window, { type: 'ws-open', id: '1', path: '/ws' });
		await waitFor(() => expect(tauriApi.wsOpen).toHaveBeenCalled());
		const win = frameOf(A).contentWindow as Window;

		unmount();

		expect(tauriApi.wsClose).toHaveBeenCalledWith('h-aone1', '1');
		vi.mocked(tauriApi.wsOpen).mockClear();
		shimMessage(win, { type: 'ws-open', id: '2', path: '/ws' });
		expect(tauriApi.wsOpen).not.toHaveBeenCalled();
	});

	it('asks for the host again after a failed lookup', async () => {
		vi.mocked(tauriApi.host).mockRejectedValueOnce(new Error('no'));
		seed(A);
		render(<ArrowAppHost />);
		act(() => useOpenApps.getState().show(A));
		await act(async () => {});
		expect(screen.queryAllByTitle(/^h-/)).toHaveLength(0);

		act(() => useOpenApps.setState({ order: [A] }));

		await waitFor(() => expect(screen.getAllByTitle(/^h-/)).toHaveLength(1));
	});
});
