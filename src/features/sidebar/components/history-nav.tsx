import type { JSX } from 'react';

import { useCanGoBack, useRouter } from '@tanstack/react-router';

import { Button } from '@/components/ui/button';

import { useTranslation } from '@/lib/i18n';

import { ArrowLeft, ArrowRight } from 'lucide-react';

const BUTTON = 'm-0.5 shrink-0 rounded-sm text-muted-foreground hover:bg-sidebar-element-hover disabled:opacity-30';

const GLYPH_SIZE = 16;

export function HistoryNav(): JSX.Element {
	const router = useRouter();
	const canGoBack = useCanGoBack();
	const { t } = useTranslation();

	return (
		<div className="flex shrink-0 items-center">
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={t('nav.back')}
				disabled={!canGoBack}
				onClick={() => router.history.back()}
				className={BUTTON}
			>
				<ArrowLeft size={GLYPH_SIZE} />
			</Button>
			<Button
				variant="ghost"
				size="icon-xs"
				aria-label={t('nav.forward')}
				onClick={() => router.history.forward()}
				className={BUTTON}
			>
				<ArrowRight size={GLYPH_SIZE} />
			</Button>
		</div>
	);
}
