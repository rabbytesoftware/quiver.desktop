import type { JSX } from 'react';

import { badgeVariants } from '@/components/ui/badge';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

import type { InferenceConfidence } from '@/domain/arrow';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import { SparklesIcon } from 'lucide-react';

interface InferredBadgeProps {
	confidence?: InferenceConfidence | null;
	/** Icon only, for a row too narrow for the word. The label stays as the accessible name. */
	compact?: boolean;
	/** Renders a real button, so it takes keyboard focus. Off inside a link, where the link is already the tab stop and a button would nest in it. */
	focusable?: boolean;
	className?: string;
}

/**
 * Marks an arrow whose manifest quiver.core built from the repository's
 * releases rather than read from one the archer wrote. A plain span on the
 * badge classes, not `<Badge>`: most callers sit inside a `<Link>`, and the
 * trigger must not add a second interactive element there.
 */
export function InferredBadge({
	confidence,
	compact = false,
	focusable = false,
	className,
}: InferredBadgeProps): JSX.Element {
	const { t } = useTranslation();
	const label = t('arrow.inferred.badge');

	const classes = cn(badgeVariants({ size: 'sm', variant: 'outline' }), 'shrink-0 gap-1', className);
	const shared = {
		'aria-label': compact ? label : undefined,
		className: classes,
		'data-confidence': confidence ?? undefined,
		'data-slot': 'inferred-badge',
	};

	return (
		<Tooltip>
			<TooltipTrigger render={focusable ? <button type="button" {...shared} /> : <span {...shared} />}>
				<SparklesIcon aria-hidden="true" className="size-2.5" />
				{!compact && label}
			</TooltipTrigger>
			<TooltipContent className="max-w-64" side="top">
				{t('arrow.inferred.tooltip')}
			</TooltipContent>
		</Tooltip>
	);
}
