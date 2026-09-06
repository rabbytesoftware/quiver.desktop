import type { JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Frame, FramePanel } from '@/components/ui/frame';

import { useTranslation } from '@/lib/i18n';

import { GlobeIcon } from 'lucide-react';

import { ConnectionRow } from './connection-row';
import type { ConnectionRowView } from '../lib/connection-rows';

export interface ConnectionPanelProps {
	loading: boolean;
	rows: ConnectionRowView[];
	openMenuId: string | null;
	onToggleMenu: (id: string) => void;
	onCloseMenu: () => void;
	onRetry: (id: string) => void;
	onSwitchToLocal: () => void;
	onSwitch: (id: string) => void;
	onRename: (id: string) => void;
	onRemove: (id: string) => void;
	onAddRemote: () => void;
}

const SKELETON_LINE =
	'h-2.5 animate-skeleton rounded-sm bg-muted bg-[length:200%_100%] [background-image:linear-gradient(90deg,var(--color-muted)_25%,color-mix(in_oklch,var(--color-muted)_35%,var(--color-background))_50%,var(--color-muted)_75%)]';

function SkeletonRows(): JSX.Element {
	return (
		<div aria-hidden="true" className="flex flex-col divide-y divide-border">
			{[96, 132, 112].map((width, index) => (
				<div className="flex items-center gap-3 py-3.5 first:pt-0 last:pb-0" key={index}>
					<div className={`size-9.5 shrink-0 rounded-lg ${SKELETON_LINE}`} />
					<div className="flex min-w-0 flex-1 flex-col gap-1.5">
						<div className={SKELETON_LINE} style={{ width }} />
						<div className={SKELETON_LINE} style={{ width: width * 1.3 }} />
					</div>
				</div>
			))}
		</div>
	);
}

export function ConnectionPanel({
	loading,
	rows,
	openMenuId,
	onToggleMenu,
	onCloseMenu,
	onRetry,
	onSwitchToLocal,
	onSwitch,
	onRename,
	onRemove,
	onAddRemote,
}: ConnectionPanelProps): JSX.Element {
	const { t } = useTranslation();
	const hasRemote = rows.some((row) => row.isRemote);

	return (
		<Frame>
			<FramePanel>
				{loading ? (
					<SkeletonRows />
				) : (
					<>
						<div className="flex max-h-[462px] flex-col divide-y divide-border overflow-y-auto">
							{rows.map((row) => (
								<ConnectionRow
									key={row.id}
									menuOpen={openMenuId === row.id}
									onCloseMenu={onCloseMenu}
									onRemove={() => onRemove(row.id)}
									onRename={() => onRename(row.id)}
									onRetry={() => onRetry(row.id)}
									onSwitch={() => onSwitch(row.id)}
									onSwitchToLocal={onSwitchToLocal}
									onToggleMenu={() => onToggleMenu(row.id)}
									row={row}
								/>
							))}
						</div>
						{!hasRemote && (
							<div className="flex flex-col items-center gap-1 px-5 pt-9 pb-2 text-center">
								<div className="mb-2.5 flex size-11 items-center justify-center rounded-xl border border-border bg-muted text-muted-foreground">
									<GlobeIcon aria-hidden="true" className="size-5" />
								</div>
								<p className="text-sm font-medium">{t('remote.empty.title')}</p>
								<p className="mb-4 max-w-80 text-xs leading-relaxed text-muted-foreground">
									{t('remote.empty.description')}
								</p>
								<Button onClick={onAddRemote} variant="outline">
									{t('remote.addButton')}
								</Button>
							</div>
						)}
					</>
				)}
			</FramePanel>
		</Frame>
	);
}
