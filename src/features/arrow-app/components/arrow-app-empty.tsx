import type { JSX } from 'react';

import { Link } from '@tanstack/react-router';

import { useTranslation } from '@/lib/i18n';

export function ArrowAppEmpty({ namespace }: { namespace: string }): JSX.Element {
	const { t } = useTranslation();

	return (
		<div className="flex h-full flex-col items-center justify-center gap-3 p-8 text-center">
			<p className="text-muted-foreground">{t('arrowApp.notRunning')}</p>
			<Link to="/arrow/$" params={{ _splat: namespace }} className="text-sm underline-offset-4 hover:underline">
				{t('arrowApp.details')}
			</Link>
		</div>
	);
}
