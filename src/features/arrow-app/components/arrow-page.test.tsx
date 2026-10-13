import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { ArrowEntry } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';

import { ArrowPage } from './arrow-page';

const detail = vi.hoisted(() => ({ data: undefined as { namespace: string } | undefined }));

vi.mock('@/lib/core-store/queries/arrow', () => ({ useArrowDetail: () => detail }));
vi.mock('./arrow-app-view', () => ({
	ArrowAppView: ({ namespace }: { namespace: string }) => <div>app of {namespace}</div>,
}));
vi.mock('@/features/arrow-details/arrow-details-screen', () => ({
	ArrowDetailsScreen: ({ namespace }: { namespace: string }) => <div>details of {namespace}</div>,
}));

const BARE = 'github.com/rabbytesoftware/quiver.chat';
const RESOLVED = `${BARE}@stable`;

function seed(namespace: string, surface: boolean): void {
	const entry = {
		namespace,
		name: 'Chat',
		version: '1.0.0',
		icon: null,
		active_run: {
			method: 'execute',
			variables: {},
			steps: [],
			surface: surface ? { mode: 'listen', path: '/', ready: true } : undefined,
		},
	} as unknown as ArrowEntry;
	useArrowStore.setState({ arrows: new Map([[namespace, entry]]) });
}

describe('ArrowPage', () => {
	beforeEach(() => {
		useArrowStore.getState().reset();
		detail.data = undefined;
	});

	it('shows the app on a bare route once the store holds the resolved ns@ref with a surface', () => {
		seed(RESOLVED, true);
		detail.data = { namespace: RESOLVED };
		render(<ArrowPage namespace={BARE} onIdentityChange={() => {}} />);

		expect(screen.getByText(`app of ${RESOLVED}`)).toBeInTheDocument();
	});

	it('shows the details page while the identity has not resolved yet', () => {
		seed(RESOLVED, true);
		render(<ArrowPage namespace={BARE} onIdentityChange={() => {}} />);

		expect(screen.getByText(`details of ${BARE}`)).toBeInTheDocument();
	});

	it('shows the details page for a resolved arrow with no surface', () => {
		seed(RESOLVED, false);
		detail.data = { namespace: RESOLVED };
		render(<ArrowPage namespace={BARE} onIdentityChange={() => {}} />);

		expect(screen.getByText(`details of ${BARE}`)).toBeInTheDocument();
	});

	it('still works for a route that already carries ns@ref', () => {
		seed(RESOLVED, true);
		detail.data = { namespace: RESOLVED };
		render(<ArrowPage namespace={RESOLVED} onIdentityChange={() => {}} />);

		expect(screen.getByText(`app of ${RESOLVED}`)).toBeInTheDocument();
	});
});
