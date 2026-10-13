import { act, render } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

const sync = vi.fn();
vi.mock('@/features/console/stores/console-controller', () => ({
	getConsoleController: () => ({ sync, submit: vi.fn() }),
}));

import { ConsoleDock } from './console-dock';

beforeEach(() => {
	sync.mockReset();
	useConnectionStore.setState({ connections: [], activeId: 'local' });
	useStatusStore.setState({ status: 'starting' });
});

describe('the console dock', () => {
	it('tells the controller which connection is active, and that it is not ready yet', () => {
		render(<ConsoleDock />);
		expect(sync).toHaveBeenLastCalledWith('local', false);
	});

	it('says so when the daemon becomes ready', () => {
		render(<ConsoleDock />);
		act(() => useStatusStore.setState({ status: 'ready' }));
		expect(sync).toHaveBeenLastCalledWith('local', true);
	});

	it('says so when it goes away again', () => {
		useStatusStore.setState({ status: 'ready' });
		render(<ConsoleDock />);
		act(() => useStatusStore.setState({ status: 'disconnected' }));
		expect(sync).toHaveBeenLastCalledWith('local', false);
	});

	it('follows the active connection', () => {
		useStatusStore.setState({ status: 'ready' });
		render(<ConsoleDock />);
		act(() => useConnectionStore.setState({ activeId: 'remote-1' }));
		expect(sync).toHaveBeenLastCalledWith('remote-1', true);
	});

	it('does not tell the controller again when nothing changed', () => {
		const { rerender } = render(<ConsoleDock />);
		const calls = sync.mock.calls.length;
		rerender(<ConsoleDock className="col-start-2" />);
		expect(sync.mock.calls).toHaveLength(calls);
	});

	it('lets the pointer through to what is under it, and sits in the column it is given', () => {
		const { container } = render(<ConsoleDock className="col-start-2" />);
		const dock = container.querySelector('[data-slot="console-dock"]') as HTMLElement;
		expect(dock.className).toContain('pointer-events-none');
		expect(dock.className).toContain('col-start-2');
		expect(dock.className).toContain('row-span-2');
	});
});
