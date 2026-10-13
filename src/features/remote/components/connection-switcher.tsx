import type { JSX } from 'react';

import { useConnectionStore } from '@/lib/connection/store';
import { useStatusStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { useRemoteStore } from '../stores/remote-store';

export function ConnectionSwitcher(): JSX.Element | null {
	const { t } = useTranslation();
	const connections = useConnectionStore((s) => s.connections);
	const activeId = useConnectionStore((s) => s.activeId);
	const status = useStatusStore((s) => s.status);
	const openCmd = useRemoteStore((s) => s.openCmd);

	// Nothing to switch to yet -- keep the rail quiet until a remote exists.
	if (connections.length <= 1) return null;

	const active = connections.find((connection) => connection.id === activeId);

	return (
		<button
			className="mr-1 inline-flex h-6 shrink-0 items-center gap-1.5 rounded-full px-1.5 text-[11.5px] font-medium hover:bg-sidebar-element-hover"
			data-status={status}
			onClick={openCmd}
			title={t('remote.switcher.label')}
			type="button"
		>
			<span
				aria-hidden="true"
				className="size-1.5 shrink-0 rounded-full bg-foreground data-[status=disconnected]:border-1.5 data-[status=disconnected]:border-destructive data-[status=disconnected]:bg-transparent data-[status=ready]:bg-foreground data-[status=starting]:animate-pulse data-[status=starting]:bg-foreground-subtle"
				data-status={status}
			/>
			<span className="max-w-[74px] truncate">{active?.name ?? t('nav.remote')}</span>
		</button>
	);
}
