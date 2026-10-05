import type { CSSProperties, JSX } from 'react';

import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { useArrowStore, useStop } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { InfoIcon, RefreshCwIcon, SquareIcon } from 'lucide-react';

interface ArrowAppHeaderProps {
	namespace: string;
	onReload: () => void;
	onDetails: () => void;
}

export function ArrowAppHeader({ namespace, onReload, onDetails }: ArrowAppHeaderProps): JSX.Element {
	const { t } = useTranslation();
	const arrow = useArrowStore((s) => s.arrows.get(namespace));
	const stop = useStop();
	const ready = arrow?.active_run?.surface?.ready ?? false;
	const name = arrow?.name ?? namespace;

	return (
		<div className="flex h-(--arrow-app-header) shrink-0 items-center gap-3 border-b border-border bg-background px-4">
			<span className="shrink-0" style={{ '--icon': '24px' } as CSSProperties}>
				<ArrowIcon icon={arrow?.icon ?? null} name={name} namespace={namespace} />
			</span>
			<span className="truncate font-medium">{name}</span>
			{arrow ? <span className="text-xs text-muted-foreground">{arrow.version}</span> : null}
			<span className="text-xs text-muted-foreground">
				{ready ? t('arrowApp.running') : t('arrowApp.starting')}
			</span>
			<div className="ml-auto flex items-center gap-2">
				<Tooltip>
					<TooltipTrigger
						render={
							<Button
								aria-label={t('arrowApp.reload')}
								onClick={onReload}
								size="icon-sm"
								variant="ghost"
							/>
						}
					>
						<RefreshCwIcon aria-hidden="true" />
					</TooltipTrigger>
					<TooltipContent side="bottom">{t('arrowApp.reload')}</TooltipContent>
				</Tooltip>
				<Button onClick={onDetails} size="sm" variant="outline">
					<InfoIcon aria-hidden="true" />
					{t('arrowApp.details')}
				</Button>
				<Button onClick={() => stop.mutate({ namespace })} size="sm" variant="destructive-outline">
					<SquareIcon aria-hidden="true" />
					{t('arrowApp.stop')}
				</Button>
			</div>
		</div>
	);
}
