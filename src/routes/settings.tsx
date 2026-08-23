import { createFileRoute } from '@tanstack/react-router';

import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

import { DeveloperSettings } from '@/features/settings/components/tabs/developer';
import { EngineSettings } from '@/features/settings/components/tabs/engine';
import { GeneralSettings } from '@/features/settings/components/tabs/general';
import { type SettingsTab, useSettingsUI } from '@/features/settings/store';
import { visibleTabs } from '@/features/settings/tabs';
import { useTranslation } from '@/lib/i18n';

const TABS: readonly string[] = ['general', 'engine', 'developer'];

export interface SettingsParams {
	tab?: SettingsTab;
}

function validateSearch(search: Record<string, unknown>): SettingsParams {
	const tab = search.tab;
	return typeof tab === 'string' && TABS.includes(tab) ? { tab: tab as SettingsTab } : {};
}

export const Route = createFileRoute('/settings')({
	validateSearch,
	component: SettingsPage,
});

function SettingsPage() {
	const { t } = useTranslation();
	const { tab } = Route.useSearch();
	const navigate = Route.useNavigate();
	const remembered = useSettingsUI((s) => s.tab);
	const setTab = useSettingsUI((s) => s.setTab);

	const tabs = visibleTabs();
	const requested = tab ?? remembered;
	const activeTab = tabs.some((item) => item.id === requested) ? requested : tabs[0].id;

	function selectTab(next: SettingsTab) {
		setTab(next);
		navigate({ search: { tab: next }, replace: true });
	}

	return (
		<Tabs
			value={activeTab}
			onValueChange={(next) => selectTab(next as SettingsTab)}
			className="flex h-full min-h-0 flex-col gap-0"
		>
			<div className="flex shrink-0 items-center border-b border-border px-3 py-1">
				<TabsList aria-label={t('settings.title')}>
					{tabs.map((item) => (
						<TabsTrigger key={item.id} value={item.id}>
							{t(item.labelKey)}
						</TabsTrigger>
					))}
				</TabsList>
			</div>

			<TabsContent value="general" className="min-h-0 flex-1 overflow-y-auto p-4">
				<GeneralSettings />
			</TabsContent>
			<TabsContent value="engine" className="min-h-0 flex-1 overflow-y-auto p-4">
				<EngineSettings />
			</TabsContent>
			{tabs.some((item) => item.id === 'developer') && (
				<TabsContent value="developer" className="min-h-0 flex-1 overflow-y-auto p-4">
					<DeveloperSettings />
				</TabsContent>
			)}
		</Tabs>
	);
}
