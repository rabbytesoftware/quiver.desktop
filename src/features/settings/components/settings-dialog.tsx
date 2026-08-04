import { Dialog, DialogClose, DialogContent, DialogTitle } from '@/components/ui/dialog';
import { Input } from '@/components/ui/input';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';

import { cn } from '@/lib/cn';
import { useMockStore } from '@/lib/mock/store';

import { type SettingsTab, useSettingsUI } from '../store';
import { ConnectionsSettings } from './tabs/connections';
import { DeveloperSettings } from './tabs/developer';

interface TabItem {
	id: SettingsTab;
	label: string;
}

/** Developer is unconditional in dev, and behind the version-tap unlock in a
 *  release build. */
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
	// A dev → release build change can leave `tab` naming a tab that is gone.
	const activeTab = tabs.some((t) => t.id === tab) ? tab : tabs[0].id;

	return (
		<Dialog open={open} onOpenChange={(next) => !next && closeSettings()}>
			{/* The stock close button is absolutely positioned for a padded dialog;
			    this one is flush, so it goes in the header row instead. */}
			<DialogContent
				showCloseButton={false}
				className="flex h-[70vh] max-h-[720px] w-[86vw] flex-col gap-0 p-0 sm:max-w-[900px]"
			>
				<div className="flex h-[38px] shrink-0 items-center justify-between gap-3 border-b border-border px-3">
					<DialogTitle className="text-sm font-medium">Settings</DialogTitle>
					<div className="flex items-center gap-2">
						<Input
							value={query}
							onChange={(e) => setQuery(e.target.value)}
							placeholder="Search settings"
							aria-label="Search settings"
							className="h-7 w-[180px]"
						/>
						<DialogClose
							aria-label="Close settings"
							className="flex size-7 items-center justify-center text-muted-foreground transition-colors hover:bg-accent hover:text-accent-foreground"
						>
							✕
						</DialogClose>
					</div>
				</div>

				<Tabs
					value={activeTab}
					onValueChange={(next) => setTab(next as SettingsTab)}
					orientation="vertical"
					className="flex min-h-0 flex-1 flex-row gap-0"
				>
					<TabsList className="h-full w-[208px] shrink-0 flex-col justify-start gap-px border-r border-border bg-transparent p-2">
						{tabs.map((item) => (
							<TabsTrigger
								key={item.id}
								value={item.id}
								className={cn(
									// flex-none and a fixed height override the stock
									// `flex-1 h-[calc(100%-1px)]`, which in a full-height
									// vertical list stretches every tab to fill the rail.
									'h-[30px] w-full flex-none justify-start px-2 text-left text-sm',
									// The selection idiom: a solid block with knocked-out
									// contents. `data-active` is what Base UI sets, and the
									// `dark:` copy is required — the stock trigger ships
									// `dark:data-active:bg-input/30`, and twMerge cannot
									// dedupe across a variant prefix the override lacks.
									'data-[active]:bg-primary data-[active]:text-primary-foreground data-[active]:shadow-none',
									'dark:data-[active]:bg-primary dark:data-[active]:text-primary-foreground'
								)}
							>
								{item.label}
							</TabsTrigger>
						))}
					</TabsList>

					<TabsContent value="connections" className="min-w-0 flex-1 overflow-y-auto p-4">
						<ConnectionsSettings />
					</TabsContent>
					{tabs.some((t) => t.id === 'developer') && (
						<TabsContent value="developer" className="min-w-0 flex-1 overflow-y-auto p-4">
							<DeveloperSettings />
						</TabsContent>
					)}
				</Tabs>
			</DialogContent>
		</Dialog>
	);
}
