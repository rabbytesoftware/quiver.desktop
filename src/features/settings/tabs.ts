import { type MessageKey } from '@/lib/i18n';

import { type SettingsTab } from './store';

export interface TabItem {
	id: SettingsTab;
	labelKey: Extract<MessageKey, `settings.tab.${string}`>;
}

export function visibleTabs(devUnlocked: boolean): TabItem[] {
	const tabs: TabItem[] = [
		{ id: 'general', labelKey: 'settings.tab.general' },
		{ id: 'connections', labelKey: 'settings.tab.connections' },
	];
	if (import.meta.env.DEV || devUnlocked) tabs.push({ id: 'developer', labelKey: 'settings.tab.developer' });
	return tabs;
}
