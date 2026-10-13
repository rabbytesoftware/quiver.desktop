import type { JSX } from 'react';

import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { FlickerSpinner } from '@/components/ui/flicker-spinner';
import { Menu, MenuItem, MenuPopup, MenuSeparator, MenuTrigger } from '@/components/ui/menu';

import { useTranslation } from '@/lib/i18n';

import {
	ArrowRightLeftIcon,
	CheckIcon,
	EllipsisVerticalIcon,
	GlobeIcon,
	MonitorIcon,
	PencilIcon,
	Trash2Icon,
	TriangleAlertIcon,
} from 'lucide-react';

import type { ConnectionRowView } from '../lib/connection-rows';

export interface ConnectionRowProps {
	row: ConnectionRowView;
	menuOpen: boolean;
	onToggleMenu: () => void;
	onCloseMenu: () => void;
	onRetry: () => void;
	onSwitchToLocal: () => void;
	onSwitch: () => void;
	onRename: () => void;
	onRemove: () => void;
}

export function ConnectionRow({
	row,
	menuOpen,
	onToggleMenu,
	onCloseMenu,
	onRetry,
	onSwitchToLocal,
	onSwitch,
	onRename,
	onRemove,
}: ConnectionRowProps): JSX.Element {
	const { t } = useTranslation();
	const Icon = row.isLocal ? MonitorIcon : GlobeIcon;

	return (
		<div className="flex items-center gap-3 py-3.5 first:pt-0 last:pb-0">
			<div
				aria-hidden="true"
				className="flex size-9.5 shrink-0 items-center justify-center rounded-lg border border-border bg-muted text-muted-foreground"
			>
				<Icon className="size-4.5" />
			</div>

			<div className="min-w-0 flex-1">
				<div className="truncate text-sm font-medium tracking-[-0.1px]">{row.name}</div>
				<div className="mt-px truncate text-xs text-muted-foreground">
					{row.isLocal ? t('remote.local.subtitle') : <span className="font-mono">{row.subtitle}</span>}
				</div>
				{row.statusKind === 'disconnected' && (
					<div className="mt-0.5 text-[11px] leading-relaxed text-destructive">
						{t('remote.status.disconnectedReason')}
					</div>
				)}
			</div>

			<div className="flex shrink-0 items-center gap-1.5">
				{row.statusKind === 'starting' && (
					<Badge className="gap-1" size="sm" variant="secondary">
						<FlickerSpinner aria-hidden="true" className="size-2.5" />
						{t('remote.status.starting')}
					</Badge>
				)}
				{row.statusKind === 'ready' && (
					<Badge className="gap-1" size="sm" variant="success">
						<CheckIcon aria-hidden="true" className="size-2.5" />
						{t('remote.status.ready')}
					</Badge>
				)}
				{row.statusKind === 'disconnected' && (
					<>
						<Badge className="gap-1" size="sm" variant="error">
							<TriangleAlertIcon aria-hidden="true" className="size-2.5" />
							{t('remote.status.disconnected')}
						</Badge>
						<Button onClick={onRetry} size="xs" variant="outline">
							{t('remote.action.retry')}
						</Button>
						{row.isRemote && (
							<Button onClick={onSwitchToLocal} size="xs" variant="ghost">
								{t('remote.action.switchToLocal')}
							</Button>
						)}
					</>
				)}
			</div>

			{row.showMenuBtn && (
				<Menu onOpenChange={(next) => (next ? onToggleMenu() : onCloseMenu())} open={menuOpen}>
					<MenuTrigger aria-label={t('remote.menu.more')} render={<Button size="icon-sm" variant="ghost" />}>
						<EllipsisVerticalIcon aria-hidden="true" />
					</MenuTrigger>
					<MenuPopup>
						{row.showConnect && (
							<MenuItem onClick={onSwitch}>
								<ArrowRightLeftIcon aria-hidden="true" />
								{t('remote.menu.switch')}
							</MenuItem>
						)}
						{row.isRemote && (
							<>
								<MenuItem onClick={onRename}>
									<PencilIcon aria-hidden="true" />
									{t('remote.menu.rename')}
								</MenuItem>
								<MenuSeparator />
								<MenuItem onClick={onRemove} variant="destructive">
									<Trash2Icon aria-hidden="true" />
									{t('remote.menu.remove')}
								</MenuItem>
							</>
						)}
					</MenuPopup>
				</Menu>
			)}
		</div>
	);
}
