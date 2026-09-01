import { useMemo, useState, type JSX, type ReactNode } from 'react';

import { useNavigate } from '@tanstack/react-router';

import { Tabs, TabsList, TabsTrigger } from '@/components/ui/tabs';

import type { InstalledVersion } from '@/domain/arrow';
import { useArrowStore } from '@/lib/core-store';
import { useArrowDetail } from '@/lib/core-store/queries/arrow';
import { useTranslation } from '@/lib/i18n';
import { splitNamespace } from '@/lib/namespace';
import { currentPlatform } from '@/lib/platform';

import { Hero } from './components/hero';
import { MetaPanel } from './components/meta-panel';
import { MethodsPanel } from './components/methods-panel';
import { ReadmePanel } from './components/readme-panel';
import { SettingsPanel } from './components/settings-panel';
import { StepsTimeline } from './components/steps-timeline';
import { buildDependencyRows } from './lib/dependency-rows';
import { CONTENT_MAX_WIDTH, CONTENT_PADDING_X } from './lib/layout';
import { groupTabs } from './lib/tab-groups';
import { useContainerWidthAtLeast } from './lib/use-container-width';

type ArrowTab = 'overview' | 'activity' | 'methods';

interface TabEntry {
	value: ArrowTab;
	label: string;
	/**
	 * Overview stays solo -- whichever of README/Details it's showing wants
	 * full width. Activity and Methods pair with each other when the
	 * container is wide; Settings lives in the rail now, not as a tab, so it
	 * never enters this at all.
	 */
	groupable: boolean;
	content: ReactNode;
}

// Wide enough that two grouped panels each get a comfortable column, not a squeeze.
const GROUP_MIN_WIDTH = 900;

// Below this, the rail drops under the tabs instead of sitting beside them --
// the same 340px rail + 28px gap breakpoint the design mock settled on, not a
// number invented here.
const RAIL_MIN_WIDTH = 860;

interface ArrowDetailsScreenProps {
	namespace: string;
}

export function ArrowDetailsScreen({ namespace }: ArrowDetailsScreenProps): JSX.Element {
	const { t } = useTranslation();
	const { data, isLoading, isError } = useArrowDetail(namespace);

	// The reactive store only ever holds arrows the user has added
	// (listeners/index.ts seeds it from `user_installed=true` only) -- for a
	// Discovered arrow, `liveEntry` stays undefined and `data`'s own
	// one-time-fetched state/active_run/last_return are used as-is, which is
	// correct: there is nothing live to overlay.
	const liveEntry = useArrowStore((state) => state.arrows.get(namespace));
	const allEntries = useArrowStore((state) => state.arrows);

	const detail = useMemo(() => {
		if (!data) return data;
		// The live overlay's `last_return` (from the WS runtime-update frame)
		// deliberately carries no `steps` -- core omits them there to avoid
		// pushing full step history on every transition (see LastReturnDetail's
		// own comment in src/domain/arrow.ts). Reuse the richer, one-time-fetched
		// `steps`/`variables` only when they're actually describing the same
		// run; a live push reporting a genuinely new outcome falls back to an
		// empty step list rather than showing stale, mismatched detail.
		const liveLastReturn = liveEntry?.last_return;
		const sameRun =
			liveLastReturn &&
			data.last_return?.method === liveLastReturn.method &&
			data.last_return?.outcome === liveLastReturn.outcome;
		return {
			...data,
			state: liveEntry?.state ?? data.state,
			active_run: liveEntry?.active_run ?? data.active_run,
			last_return: liveLastReturn
				? {
						...liveLastReturn,
						variables: sameRun ? data.last_return!.variables : {},
						steps: sameRun ? data.last_return!.steps : [],
					}
				: data.last_return,
		};
	}, [data, liveEntry]);

	const versions = useMemo<InstalledVersion[]>(() => {
		if (!detail) return [];
		const { head: base } = splitNamespace(detail.namespace);
		const matches: InstalledVersion[] = [];
		for (const entry of allEntries.values()) {
			const { head, tail } = splitNamespace(entry.namespace);
			if (head === base && tail) matches.push({ ref: tail.slice(1), version: entry.version, state: entry.state });
		}
		return matches;
	}, [allEntries, detail]);

	const platform = useMemo(() => currentPlatform(), []);

	const [tab, setTab] = useState<ArrowTab>('overview');
	const [values, setValues] = useState<Record<string, string>>({});
	const [seededFor, setSeededFor] = useState<string | null>(null);
	const [initialTabSetFor, setInitialTabSetFor] = useState<string | null>(null);
	const [wasRunning, setWasRunning] = useState(false);
	const navigate = useNavigate();
	const [isWide, tabsContainerRef] = useContainerWidthAtLeast(GROUP_MIN_WIDTH);
	const [isRailWide, railContainerRef] = useContainerWidthAtLeast(RAIL_MIN_WIDTH);

	// Seed the local settings store from manifest defaults, once per arrow --
	// there is no "current saved value" to read from core (§7.3): the only
	// source of truth for a starting value is the manifest's own default.
	// Adjusted during render rather than in an effect (React's documented
	// "adjusting state when a prop changes" pattern) so the very first paint
	// for a newly-loaded arrow already shows its defaults.
	if (data && seededFor !== data.namespace) {
		setSeededFor(data.namespace);
		const defaults: Record<string, string> = {};
		for (const variable of data.variables) {
			if (variable.default !== undefined) defaults[variable.name] = variable.default;
		}
		setValues(defaults);
	}

	// Default tab: Activity when there's something to show on load (a run in
	// flight, or the last one failed); Overview otherwise. Runs once per
	// arrow, not on every re-render once `detail` starts changing live -- and
	// always explicitly resets to one or the other, so switching from arrow A
	// (left on Methods, say) to arrow B doesn't carry A's tab selection over.
	if (detail && initialTabSetFor !== detail.namespace) {
		setInitialTabSetFor(detail.namespace);
		setTab(detail.active_run !== null || detail.last_return?.outcome === 'failed' ? 'activity' : 'overview');
	}

	// Auto-switch to Activity the moment a run starts (active_run transitions
	// null -> non-null) -- but only that one transition. A user who
	// deliberately navigates away from Activity while a run is in flight
	// should not get yanked back on the next WS frame.
	const running = detail?.active_run !== null && detail?.active_run !== undefined;
	if (running !== wasRunning) {
		setWasRunning(running);
		if (running) setTab('activity');
	}

	function handleValueChange(name: string, value: string): void {
		setValues((current) => ({ ...current, [name]: value }));
	}

	function handleVersionChange(ref: string): void {
		// Only ever reachable via the Hero, which itself only renders after the
		// `!detail` guard below has already passed -- `detail!` reflects that
		// real invariant rather than adding a branch TypeScript needs but no
		// test can actually reach through this component's own render tree.
		const { head: base } = splitNamespace(detail!.namespace);
		void navigate({ to: '/arrow/$', params: { _splat: `${base}@${ref}` } });
	}

	if (isLoading) {
		return <div className="p-6 text-sm text-muted-foreground">{t('arrow.loading')}</div>;
	}
	if (isError || !detail) {
		return <div className="p-6 text-sm text-muted-foreground">{t('arrow.error')}</div>;
	}

	const target = detail.targets.find((t2) => t2.platform === platform) ?? detail.targets[0];
	const methods = target?.methods ?? [];
	// The rail is the ONLY place Details and Settings live -- never also as a
	// tab, so there is exactly one copy of each on screen. Details covers for
	// Overview when Overview is spending its own slot on the README; Settings
	// shows whenever the arrow declares variables, independent of that.
	const hasDetailsRail = !!detail.readme;
	const hasSettingsRail = detail.user_installed && detail.variables.length > 0;
	const hasRail = hasDetailsRail || hasSettingsRail;

	// Core's dependency-graph endpoints only ever return bare namespace@ref
	// strings -- this fills in a name/icon/live state from the reactive
	// catalog wherever the user happens to have that arrow locally.
	const dependsOnRows = buildDependencyRows(
		detail.dependencies.map((dependency) => dependency.namespace),
		allEntries
	);
	const requiredByRows = buildDependencyRows(detail.dependents, allEntries);

	const entries: TabEntry[] = [
		{
			value: 'overview',
			label: t('arrow.tab.overview'),
			groupable: false,
			content: detail.readme ? (
				<ReadmePanel readme={detail.readme} />
			) : (
				<MetaPanel
					credits={detail.credits}
					dependsOn={dependsOnRows}
					maintainers={detail.maintainers}
					netbridge={detail.netbridge}
					requirement={target?.requirement}
					requiredBy={requiredByRows}
					url={detail.url}
				/>
			),
		},
		...(detail.user_installed
			? [
					{
						value: 'activity' as const,
						label: t('arrow.tab.activity'),
						groupable: true,
						content: (
							<StepsTimeline
								activeRun={detail.active_run}
								lastReturn={detail.last_return}
								userInstalled={detail.user_installed}
							/>
						),
					},
				]
			: []),
		{
			value: 'methods',
			label: t('arrow.tab.methods'),
			groupable: true,
			content: (
				<MethodsPanel
					methods={methods}
					onValueChange={handleValueChange}
					values={values}
					variables={detail.variables}
				/>
			),
		},
	];

	const groups = groupTabs(entries);
	const activeGroup = groups.find((group) => group.some((entry) => entry.value === tab)) ?? groups[0];
	const visibleEntries = isWide ? activeGroup : activeGroup.filter((entry) => entry.value === tab);

	return (
		<div className="flex flex-col">
			<Hero
				detail={{ ...detail, versions }}
				onValueChange={handleValueChange}
				onVersionChange={handleVersionChange}
				platform={platform}
				values={values}
			/>

			<div
				className={`${CONTENT_PADDING_X} ${CONTENT_MAX_WIDTH} pb-12`}
				data-testid="arrow-detail-content"
				ref={railContainerRef}
			>
				<div
					className={
						hasRail && isRailWide
							? 'grid grid-cols-[minmax(0,1fr)_340px] items-start gap-7'
							: hasRail
								? 'flex flex-col gap-7'
								: undefined
					}
					data-testid="arrow-detail-layout"
				>
					<div data-testid="arrow-tabs-container" ref={tabsContainerRef}>
						<Tabs onValueChange={(value) => setTab(value as ArrowTab)} value={tab}>
							<TabsList>
								{isWide
									? groups.map((group) => {
											const value =
												group.find((entry) => entry.value === tab)?.value ?? group[0].value;
											const label =
												group.length === 2
													? t('arrow.tab.group', { a: group[0].label, b: group[1].label })
													: group[0].label;
											return (
												<TabsTrigger
													key={group.map((entry) => entry.value).join('+')}
													value={value}
												>
													{label}
												</TabsTrigger>
											);
										})
									: entries.map((entry) => (
											<TabsTrigger key={entry.value} value={entry.value}>
												{entry.label}
											</TabsTrigger>
										))}
							</TabsList>
						</Tabs>

						<div
							className={visibleEntries.length === 2 ? 'mt-4 grid grid-cols-2 items-start gap-4' : 'mt-4'}
						>
							{visibleEntries.map((entry) => (
								<div key={entry.value}>{entry.content}</div>
							))}
						</div>
					</div>

					{hasRail && (
						<div className="flex flex-col gap-7">
							{hasSettingsRail && (
								<SettingsPanel
									onChange={handleValueChange}
									values={values}
									variables={detail.variables}
								/>
							)}
							{hasDetailsRail && (
								<MetaPanel
									credits={detail.credits}
									dependsOn={dependsOnRows}
									maintainers={detail.maintainers}
									netbridge={detail.netbridge}
									requirement={target?.requirement}
									requiredBy={requiredByRows}
									url={detail.url}
								/>
							)}
						</div>
					)}
				</div>
			</div>
		</div>
	);
}
