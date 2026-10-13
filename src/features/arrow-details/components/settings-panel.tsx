import type { JSX } from 'react';

import { Badge } from '@/components/ui/badge';
import { Frame, FrameHeader, FramePanel, FrameTitle } from '@/components/ui/frame';

import type { ArrowVariable } from '@/domain/arrow';
import { useTranslation } from '@/lib/i18n';

import { VariablesFieldList } from './variable-field';

interface SettingsPanelProps {
	variables: ArrowVariable[];
	values: Record<string, string>;
	onChange: (name: string, value: string) => void;
}

/**
 * Every variable this arrow's manifest declares, editable in place. Lives in
 * the details rail -- its one and only home, so it's never duplicated
 * elsewhere on the page. Same fields, same behavior as the "Configure"
 * dialog opened from a step preview, just inline instead of behind a click.
 */
export function SettingsPanel({ variables, values, onChange }: SettingsPanelProps): JSX.Element {
	const { t } = useTranslation();

	const sensitiveCount = variables.filter((variable) => variable.sensitive).length;
	const summaryPlural = t('arrow.settings.summary', { count: variables.length });
	const summary =
		sensitiveCount > 0
			? t('arrow.settings.summarySensitive', { sensitive: sensitiveCount, summary: summaryPlural })
			: summaryPlural;

	return (
		<Frame>
			<FrameHeader>
				<FrameTitle className="flex items-center gap-2">
					{t('arrow.tab.settings')}
					<Badge size="sm" variant="secondary">
						{summary}
					</Badge>
				</FrameTitle>
			</FrameHeader>
			<FramePanel>
				<VariablesFieldList onChange={onChange} values={values} variables={variables} />
			</FramePanel>
		</Frame>
	);
}
