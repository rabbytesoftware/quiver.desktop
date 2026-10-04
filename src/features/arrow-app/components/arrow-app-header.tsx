import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import { useArrowStore, useStop } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

interface ArrowAppHeaderProps {
	namespace: string;
	onReload: () => void;
}

export function ArrowAppHeader({ namespace, onReload }: ArrowAppHeaderProps): JSX.Element {
	const { t } = useTranslation();
	const arrow = useArrowStore((s) => s.arrows.get(namespace));
	const stop = useStop();
	const ready = arrow?.active_run?.surface?.ready ?? false;

	return (
		<div className="flex h-(--arrow-app-header) items-center gap-3 border-b border-border bg-background px-4">
			{arrow?.icon ? <img src={arrow.icon} alt="" className="size-6 rounded" /> : null}
			<span className="truncate font-medium">{arrow?.name ?? namespace}</span>
			{arrow ? <span className="text-xs text-muted-foreground">{arrow.version}</span> : null}
			<span className="text-xs text-muted-foreground">{ready ? t('arrowApp.running') : t('arrowApp.starting')}</span>
			<div className="ml-auto flex items-center gap-2">
				<Button variant="ghost" size="sm" onClick={onReload}>
					{t('arrowApp.reload')}
				</Button>
				<Button variant="ghost" size="sm" onClick={() => stop.mutate({ namespace })}>
					{t('arrowApp.stop')}
				</Button>
				<Link
					to="/arrow/$"
					params={{ _splat: namespace }}
					className="text-sm text-muted-foreground hover:text-foreground"
				>
					{t('arrowApp.details')}
				</Link>
			</div>
		</div>
	);
}
