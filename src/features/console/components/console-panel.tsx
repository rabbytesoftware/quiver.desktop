import type { JSX } from 'react';

import { getConsoleController } from '@/features/console/stores/console-controller';
import { cn } from '@/lib/cn';
import { useTranslation } from '@/lib/i18n';

import { ConsoleLog } from './console-log';
import { ConsolePrompt } from './console-prompt';
import { useConsoleStore } from '../stores/console-store';

/**
 * The quake-style console: it drops from the top of the content, shows the
 * connected daemon's log, and takes commands. Nothing else -- no title bar, no
 * filters; closing it is the build indicator's job (or Escape).
 *
 * A daemon with no console gets one line saying so, instead of controls that
 * cannot work.
 */
export function ConsolePanel(): JSX.Element {
	const { t } = useTranslation();
	const open = useConsoleStore((s) => s.open);
	const support = useConsoleStore((s) => s.support);
	const entries = useConsoleStore((s) => s.entries);
	const expandedId = useConsoleStore((s) => s.expandedId);
	const toggleExpanded = useConsoleStore((s) => s.toggleExpanded);
	const tail = useConsoleStore((s) => s.tail);

	return (
		<section
			id="console-panel"
			aria-label={t('console.panel.label')}
			// Closed means unreachable, not merely off-screen: nothing in it may take focus.
			inert={!open}
			data-open={open}
			className={cn(
				'pointer-events-auto absolute inset-x-0 top-0 z-20 flex h-[min(26.25rem,70%)] flex-col border-b bg-console text-console-foreground transition-[transform,visibility] duration-200 ease-out',
				open ? 'visible translate-y-0' : 'invisible -translate-y-full'
			)}
		>
			{support === 'unsupported' ? (
				<p className="m-auto px-6 text-center text-xs text-console-dim">{t('console.unsupported')}</p>
			) : (
				<>
					<ConsoleLog
						entries={entries}
						expandedId={expandedId}
						onToggle={toggleExpanded}
						revealKey={open}
						tailKey={tail}
					/>
					<ConsolePrompt onSubmit={(line) => getConsoleController().submit(line)} focused={open} />
				</>
			)}
		</section>
	);
}
