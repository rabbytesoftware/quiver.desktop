import type { JSX } from 'react';

import type { InferenceConfidence } from '@/domain/arrow';
import { confidenceKey, warningKey } from '@/features/arrow-details/lib/inference';
import { useTranslation } from '@/lib/i18n';

interface InferenceNotesProps {
	confidence: InferenceConfidence | null | undefined;
	warnings: string[];
}

/** What an inferred manifest is unsure about; the "Auto-made" badge itself is `InferredBadge`. */
export function InferenceNotes({ confidence, warnings }: InferenceNotesProps): JSX.Element | null {
	const { t } = useTranslation();
	if (!confidence && warnings.length === 0) return null;

	return (
		<div className="mt-3 max-w-2xl text-xs text-muted-foreground">
			{confidence && (
				<p>
					{t('arrow.inferred.confidence', {
						confidence: t(confidenceKey(confidence) ?? 'arrow.inferred.confidence.low'),
					})}
				</p>
			)}
			{warnings.length > 0 && (
				<>
					<p className="mt-1 font-medium text-foreground">{t('arrow.inferred.warnings')}</p>
					<ul className="mt-1 list-disc space-y-0.5 pl-4">
						{warnings.map((code) => {
							const key = warningKey(code);
							return <li key={code}>{key ? t(key) : code}</li>;
						})}
					</ul>
				</>
			)}
		</div>
	);
}
