import type { JSX } from 'react';

import { useTranslation } from '@/lib/i18n';

import { useDesktopBuild } from '../hooks/use-desktop-build';
import { describeBuildLong, describeCore, formatBuild, type BuildDescriptor } from '../lib/build-info';
import { useConsoleStore } from '../stores/console-store';

// The full line needs about 160px (label + `nightly MM-DD HH:mm` at 10px mono);
// below that the time of day is dropped. Written out, not built from a number:
// Tailwind only generates classes it can read in the source.
const FULL = '@max-[159px]:hidden';
const COMPACT = 'hidden @max-[159px]:inline';
const REVEAL = 'group-hover/indicator:hidden group-focus-visible/indicator:hidden';

function Line({ label, build }: { label: string; build: BuildDescriptor | null }): JSX.Element {
	if (build === null) {
		return (
			<span className="flex gap-2 whitespace-nowrap">
				<span className="w-[26px] shrink-0 opacity-70">{label}</span>
				<span aria-hidden="true">—</span>
			</span>
		);
	}

	const rest = formatBuild(build);
	const compact = formatBuild(build, { compact: true });
	const hover = formatBuild(build, { hover: true });

	return (
		<span className="flex gap-2 whitespace-nowrap">
			<span className="w-[26px] shrink-0 opacity-70">{label}</span>
			<span className={`truncate ${REVEAL}`}>
				{rest === compact ? (
					rest
				) : (
					<>
						<span className={FULL}>{rest}</span>
						<span className={COMPACT}>{compact}</span>
					</>
				)}
			</span>
			{hover !== rest && (
				<span className="hidden truncate group-hover/indicator:inline group-focus-visible/indicator:inline">
					{hover}
				</span>
			)}
		</span>
	);
}

/**
 * Which builds are running, and the way into the console.
 *
 * Two lines, `core` (the daemon) and `app` (this desktop). At rest a rolling
 * build shows when it was built and a release shows its version; on hover each
 * shows its commit. No background until it is hovered or the console is open.
 */
export function BuildIndicator(): JSX.Element {
	const { t } = useTranslation();
	const open = useConsoleStore((s) => s.open);
	const toggle = useConsoleStore((s) => s.toggle);
	const versions = useConsoleStore((s) => s.versions);

	const app = useDesktopBuild();
	const core = describeCore(versions);

	const details = [
		core && `quiver.core: ${describeBuildLong(core)}`,
		app && `quiver.desktop: ${describeBuildLong(app)}`,
	]
		.filter((line): line is string => Boolean(line))
		.join('\n');

	return (
		<div data-tauri-drag-region data-slot="build-indicator" className="@container flex min-w-0 flex-1 items-center">
			<button
				type="button"
				aria-label={t('console.indicator.label')}
				aria-expanded={open}
				aria-controls="console-panel"
				title={details || undefined}
				onClick={toggle}
				className="group/indicator flex h-7 max-w-full min-w-0 flex-col justify-center rounded-md px-1.5 font-mono text-[10px] leading-3 text-muted-foreground outline-none hover:bg-sidebar-element-hover hover:text-foreground focus-visible:ring-2 focus-visible:ring-ring aria-expanded:bg-sidebar-element-hover aria-expanded:text-foreground"
			>
				<Line label={t('console.indicator.core')} build={core} />
				<Line label={t('console.indicator.app')} build={app} />
			</button>
		</div>
	);
}
