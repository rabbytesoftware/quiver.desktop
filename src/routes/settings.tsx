import { createFileRoute } from '@tanstack/react-router';

import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

import { ConnectionsSettings } from '@/features/settings/components/tabs/connections';
import { DeveloperSettings } from '@/features/settings/components/tabs/developer';
import { GeneralSettings } from '@/features/settings/components/tabs/general';
import { type SettingsTab, useSettingsUI } from '@/features/settings/store';
import { visibleTabs } from '@/features/settings/tabs';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';
import { useMockStore } from '@/lib/mock/store';

const TABS: readonly string[] = ['general', 'connections', 'developer'];

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
	const query = useSettingsUI((s) => s.query);
	const setTab = useSettingsUI((s) => s.setTab);
	const setQuery = useSettingsUI((s) => s.setQuery);
	const devUnlocked = useMockStore((s) => s.devUnlocked);

	const tabs = visibleTabs(devUnlocked);
	const requested = tab ?? remembered;
	const activeTab = tabs.some((item) => item.id === requested) ? requested : tabs[0].id;

	function selectTab(next: SettingsTab) {
		setTab(next);
		navigate({ search: { tab: next }, replace: true });
	}

	return (
		<div className="flex h-full min-h-0 flex-col">
			<div className="flex h-[38px] shrink-0 items-center justify-between gap-3 border-b border-border px-3">
				<h1 className="text-sm font-medium">{t('settings.title')}</h1>
				<Input
					value={query}
					onChange={(e) => setQuery(e.target.value)}
					placeholder={t('settings.search.placeholder')}
					aria-label={t('settings.search.label')}
					className="h-7 w-[180px]"
				/>
			</div>

			<Tabs
				value={activeTab}
				onValueChange={(next) => selectTab(next as SettingsTab)}
				orientation="vertical"
				className="flex min-h-0 flex-1 flex-row gap-0"
			>
				<TabsList className="h-full w-[208px] shrink-0 flex-col justify-start gap-px border-r border-border bg-transparent p-2">
					{tabs.map((item) => (
						<TabsTrigger
							key={item.id}
							value={item.id}
							className={cn(
								'h-[30px] w-full flex-none justify-start px-2 text-left text-sm',
								'data-[active]:bg-primary data-[active]:text-primary-foreground data-[active]:shadow-none',
								'dark:data-[active]:bg-primary dark:data-[active]:text-primary-foreground'
							)}
						>
							{t(item.labelKey)}
						</TabsTrigger>
					))}
				</TabsList>

				<TabsContent value="general" className="min-w-0 flex-1 overflow-y-auto p-4">
					<GeneralSettings />
				</TabsContent>
				<TabsContent value="connections" className="min-w-0 flex-1 overflow-y-auto p-4">
					<ConnectionsSettings />
				</TabsContent>
				{tabs.some((item) => item.id === 'developer') && (
					<TabsContent value="developer" className="min-w-0 flex-1 overflow-y-auto p-4">
						<DeveloperSettings />
					</TabsContent>
				)}
			</Tabs>
		</div>
	);
}
