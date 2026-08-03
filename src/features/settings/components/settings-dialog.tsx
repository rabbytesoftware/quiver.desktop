import { TextInput } from '@/components/ui/controls';

import { cn } from '@/lib/cn';
import { useMockStore } from '@/lib/mock/store';

import { Dialog } from '@base-ui-components/react/dialog';
import { Tabs } from '@base-ui-components/react/tabs';

import { type SettingsTab, useSettingsUI } from '../store';
import { ConnectionsSettings } from './tabs/connections';
import { DeveloperSettings } from './tabs/developer';

interface TabItem {
	id: SettingsTab;
	label: string;
}

/**
 * Which tabs exist right now.
 *
 * Two, not six. `Appearance` waits until the light theme has been reviewed on a
 * real screen, and everything else the app will eventually want to configure
 * does not exist yet — a settings dialog full of placeholder panels is worse
 * than a small one.
 *
 * Developer is unconditional in dev and gated behind the version-tap unlock in
 * a release build, per `VersionUnlock`.
 */
export function visibleTabs(devUnlocked: boolean): TabItem[] {
	const tabs: TabItem[] = [{ id: 'connections', label: 'Connections' }];
	if (import.meta.env.DEV || devUnlocked) tabs.push({ id: 'developer', label: 'Developer' });
	return tabs;
}

export function SettingsDialog() {
	const open = useSettingsUI((s) => s.open);
	const tab = useSettingsUI((s) => s.tab);
	const query = useSettingsUI((s) => s.query);
	const setTab = useSettingsUI((s) => s.setTab);
	const setQuery = useSettingsUI((s) => s.setQuery);
	const closeSettings = useSettingsUI((s) => s.closeSettings);
	const devUnlocked = useMockStore((s) => s.devUnlocked);

	const tabs = visibleTabs(devUnlocked);
	// The unlock can be revoked by a build change (dev → release), so a
	// persisted `tab: 'developer'` can name a tab that is no longer there.
	const activeTab = tabs.some((t) => t.id === tab) ? tab : tabs[0].id;

	return (
		<Dialog.Root open={open} onOpenChange={(next) => !next && closeSettings()}>
			<Dialog.Portal>
				<Dialog.Backdrop className="fixed inset-0 z-40 bg-black/50" />
				<Dialog.Popup
					className={cn(
						'fixed left-1/2 top-1/2 z-50 flex h-[70vh] max-h-[720px] w-[86vw] max-w-[900px]',
						'-translate-x-1/2 -translate-y-1/2 flex-col border border-line bg-ground text-ink shadow-2xl'
					)}
				>
					<div className="flex h-[38px] shrink-0 items-center justify-between gap-3 border-b border-line px-3">
						<Dialog.Title className="text-[13px] font-medium">Settings</Dialog.Title>
						<div className="flex items-center gap-2">
							<TextInput
								value={query}
								onChange={setQuery}
								placeholder="Search settings"
								aria-label="Search settings"
								className="w-[180px]"
							/>
							<Dialog.Close
								className="flex h-[26px] w-[26px] items-center justify-center text-ink-2 hover:bg-hover hover:text-ink"
								aria-label="Close settings"
							>
								✕
							</Dialog.Close>
						</div>
					</div>

					<Tabs.Root
						value={activeTab}
						onValueChange={(next) => setTab(next as SettingsTab)}
						orientation="vertical"
						className="flex min-h-0 flex-1"
					>
						{/* 208px, the same width as the app's own rail — the dialog is not
						    a different piece of software. Base UI owns the roving tabindex
						    and the arrow/Home/End keys that Crowbar hand-rolls. */}
						<Tabs.List className="flex w-[208px] shrink-0 flex-col gap-px border-r border-line p-2">
							{tabs.map((item) => (
								<Tabs.Tab
									key={item.id}
									value={item.id}
									className={cn(
										'flex h-[30px] cursor-default select-none items-center px-2 text-left text-[13px] transition-colors',
										'text-ink-2 hover:bg-hover hover:text-ink',
										// The selection idiom: a solid block with knocked-out
										// contents, identical to the rail's.
										// `data-active`, NOT `data-selected` — that is the
										// attribute Base UI's Tabs actually sets. The wrong
										// one matches nothing and fails silently: a rail
										// where no tab ever looks selected.
										'data-[active]:bg-fill data-[active]:text-fill-ink',
										'focus-visible:outline focus-visible:outline-1 focus-visible:-outline-offset-1 focus-visible:outline-ring'
									)}
								>
									{item.label}
								</Tabs.Tab>
							))}
						</Tabs.List>

						<Tabs.Panel value="connections" className="min-w-0 flex-1 overflow-y-auto p-4">
							<ConnectionsSettings />
						</Tabs.Panel>
						{tabs.some((t) => t.id === 'developer') && (
							<Tabs.Panel value="developer" className="min-w-0 flex-1 overflow-y-auto p-4">
								<DeveloperSettings />
							</Tabs.Panel>
						)}
					</Tabs.Root>
				</Dialog.Popup>
			</Dialog.Portal>
		</Dialog.Root>
	);
}
