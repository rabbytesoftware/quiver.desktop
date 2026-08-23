import { type SettingsTab } from '@/features/settings/stores/settings-store';
import { type MessageKey } from '@/lib/i18n';

export interface TabItem {
	id: SettingsTab;
	labelKey: Extract<MessageKey, `settings.tab.${string}`>;
}

export function visibleTabs(): TabItem[] {
	const tabs: TabItem[] = [
		{ id: 'general', labelKey: 'settings.tab.general' },
		{ id: 'engine', labelKey: 'settings.tab.engine' },
	];
	if (import.meta.env.DEV) tabs.push({ id: 'developer', labelKey: 'settings.tab.developer' });
	return tabs;
}
