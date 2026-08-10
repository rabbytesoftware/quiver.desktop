import { Link } from '@tanstack/react-router';

import { useTranslation } from '@/lib/i18n';
import { currentMock } from '@/lib/mock';
import { getScenario } from '@/lib/mock/world/scenarios';

export function MockIndicator() {
	const { t } = useTranslation();
	const mock = currentMock();
	if (!mock) return null;

	const label = getScenario(mock.world.scenario).label;

	return (
		<div
			data-tauri-drag-region
			className="flex h-[22px] shrink-0 select-none items-center justify-center gap-2 bg-primary px-3 text-primary-foreground"
		>
			<span className="text-[11px] font-medium uppercase tracking-[0.12em]">{t('mock.badge')}</span>
			<span className="text-[11px] opacity-70">{t('mock.status', { scenario: label })}</span>
			<Link
				to="/settings"
				search={{ tab: 'developer' }}
				className="ml-1 text-[11px] underline underline-offset-2 opacity-70 hover:opacity-100"
			>
				{t('mock.turnOff')}
			</Link>
		</div>
	);
}
