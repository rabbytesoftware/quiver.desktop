import { useEffect, useState, type JSX } from 'react';

import { Dialog, DialogPopup, DialogTitle } from '@/components/ui/dialog';
import { Spinner } from '@/components/ui/spinner';

import { ArrowDetailsScreen } from '@/features/arrow-details/arrow-details-screen';
import { useArrowStore } from '@/lib/core-store';
import { useTranslation } from '@/lib/i18n';

import { ArrowAppHeader } from './arrow-app-header';
import { useOpenApps } from '../store/open-apps';

interface ArrowAppViewProps {
	namespace: string;
	onIdentityChange: (namespace: string) => void;
}

/**
 * What the arrow page shows while the arrow has an interface: the header and,
 * under it, the kept-alive frame the host draws over this area once ready.
 */
export function ArrowAppView({ namespace, onIdentityChange }: ArrowAppViewProps): JSX.Element {
	const { t } = useTranslation();
	const ready = useArrowStore((s) => s.arrows.get(namespace)?.active_run?.surface?.ready ?? false);
	const [detailsOpen, setDetailsOpen] = useState(false);

	useEffect(() => {
		useOpenApps.getState().show(namespace);
		return () => useOpenApps.getState().hide();
	}, [namespace]);

	return (
		<div className="flex h-full flex-col">
			<ArrowAppHeader
				namespace={namespace}
				onDetails={() => setDetailsOpen(true)}
				onReload={() => useOpenApps.getState().reload(namespace)}
			/>
			{!ready && (
				<div
					aria-busy="true"
					className="flex flex-1 items-center justify-center gap-2 text-sm text-muted-foreground"
					role="status"
				>
					<Spinner aria-hidden="true" className="size-4" />
					{t('arrowApp.starting')}
				</div>
			)}
			<Dialog onOpenChange={setDetailsOpen} open={detailsOpen}>
				<DialogPopup className="h-[90vh] max-w-5xl">
					<DialogTitle className="sr-only">{t('arrowApp.details')}</DialogTitle>
					<div className="min-h-0 flex-1 overflow-auto">
						<ArrowDetailsScreen namespace={namespace} onIdentityChange={onIdentityChange} />
					</div>
				</DialogPopup>
			</Dialog>
		</div>
	);
}
