import { type MessageKey } from '@/lib/i18n';

import { type SettingsTab } from './store';

export interface TabItem {
	id: SettingsTab;
	/**
	 * A key, not a label: `visibleTabs` is a pure function with no locale of its
	 * own, and resolving here would pin the tab names to whatever language was
	 * in force when it was called.
	 *
	 * Narrowed to the `settings.tab.*` keys rather than typed `MessageKey`, and
	 * that is not cosmetic: `t` derives its parameter list from the key, so a
	 * `MessageKey`-typed argument makes it the union of EVERY message's
	 * parameters — including the ones that require a `count` — and `t(item.labelKey)`
	 * stops compiling. Any narrow union of parameterless keys works; deriving it
	 * means a fourth tab needs no edit here.
	 */
	labelKey: Extract<MessageKey, `settings.tab.${string}`>;
}

/** Developer is unconditional in dev, and behind the version-tap unlock in a
 *  release build. */
export function visibleTabs(devUnlocked: boolean): TabItem[] {
	const tabs: TabItem[] = [
		{ id: 'general', labelKey: 'settings.tab.general' },
		{ id: 'connections', labelKey: 'settings.tab.connections' },
	];
	if (import.meta.env.DEV || devUnlocked) tabs.push({ id: 'developer', labelKey: 'settings.tab.developer' });
	return tabs;
}
