import { useEffect, type JSX } from 'react';

import { getConsoleController } from '@/features/console/stores/console-controller';
import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';

import { ConsolePanel } from './console-panel';

/**
 * Hosts the console in the content column and keeps it in step with the
 * connection: whichever daemon is active, once it is ready, is the one whose
 * capabilities are read and whose log is shown.
 *
 * The pointer passes through the dock itself, so only the panel (when it is
 * down) takes clicks.
 */
export function ConsoleDock({ className }: { className?: string }): JSX.Element {
	const activeId = useConnectionStore((s) => s.activeId);
	const status = useStatusStore((s) => s.status);

	useEffect(() => {
		getConsoleController().sync(activeId, status === 'ready');
	}, [activeId, status]);

	return (
		<div
			data-slot="console-dock"
			className={`pointer-events-none relative row-span-2 row-start-1 min-h-0 overflow-hidden ${className ?? ''}`}
		>
			<ConsolePanel />
		</div>
	);
}
