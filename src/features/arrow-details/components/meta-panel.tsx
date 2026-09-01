import type { JSX, ReactNode } from 'react';

import { Link } from '@tanstack/react-router';

import { Badge } from '@/components/ui/badge';
import { Frame, FrameHeader, FramePanel, FrameTitle } from '@/components/ui/frame';

import type { ArrowCredit, ArrowPort, ArrowRequirement } from '@/domain/arrow';
import type { DependencyRow } from '@/features/arrow-details/lib/dependency-rows';
import { computeStatus, STATUS_BADGE_VARIANT, STATUS_ICONS } from '@/features/arrow-details/lib/status';
import { ArrowIcon } from '@/features/sidebar/components/arrows/arrow-icon';
import { useTranslation } from '@/lib/i18n';

import { CpuIcon, HardDriveIcon, LinkIcon, MemoryStickIcon, type LucideIcon } from 'lucide-react';

interface MetaPanelProps {
	requirement?: ArrowRequirement;
	netbridge: ArrowPort[];
	maintainers: ArrowCredit[];
	credits: ArrowCredit[];
	url: string;
	dependsOn?: DependencyRow[];
	requiredBy?: DependencyRow[];
}

interface SectionProps {
	title: string;
	children: ReactNode;
}

function Section({ title, children }: SectionProps): JSX.Element {
	return (
		<section className="py-4 first:pt-0 last:pb-0">
			<h3 className="mb-2.5 text-xs font-medium text-muted-foreground">{title}</h3>
			<div className="flex flex-col">{children}</div>
		</section>
	);
}

interface RequirementRowProps {
	icon: LucideIcon;
	label: string;
	value: string;
}

function RequirementRow({ icon: Icon, label, value }: RequirementRowProps): JSX.Element {
	return (
		<div className="flex items-center gap-2.5 py-1.5">
			<Icon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
			<span className="min-w-0 flex-1 text-sm">{label}</span>
			<span className="shrink-0 font-mono text-sm text-muted-foreground">{value}</span>
		</div>
	);
}

interface PortRowProps {
	port: ArrowPort;
	requiredLabel: string;
}

function PortRow({ port, requiredLabel }: PortRowProps): JSX.Element {
	return (
		<div className="flex items-center gap-2.5 py-1.5">
			<span className="min-w-0 flex-1 truncate font-mono text-sm">{port.name}</span>
			<Badge className="uppercase" size="sm" variant="outline">
				{port.protocol}
			</Badge>
			{port.required && (
				<Badge size="sm" variant="secondary">
					{requiredLabel}
				</Badge>
			)}
			<span className="shrink-0 font-mono text-sm text-muted-foreground">{port.default}</span>
		</div>
	);
}

interface CreditRowProps {
	credit: ArrowCredit;
}

/** A stable identity for a credit row that doesn't depend on its position in the list. */
function creditKey(credit: ArrowCredit): string {
	return `${credit.name}|${credit.email ?? ''}|${credit.url ?? ''}`;
}

/** Same row shape for both Maintainers and Credits -- an initial badge, then the name, linked when a url is present. */
function CreditRow({ credit }: CreditRowProps): JSX.Element {
	const initial = credit.name.charAt(0).toUpperCase();

	return (
		<div className="flex items-center gap-2.5 py-1.5">
			<span
				aria-hidden="true"
				className="flex size-6 shrink-0 items-center justify-center rounded-full bg-muted text-xs font-medium text-muted-foreground"
			>
				{initial}
			</span>
			{credit.url ? (
				<a
					className="min-w-0 flex-1 break-words text-sm underline-offset-4 hover:underline"
					href={credit.url}
					rel="noreferrer"
					target="_blank"
				>
					{credit.name}
				</a>
			) : (
				<span className="min-w-0 flex-1 break-words text-sm">{credit.name}</span>
			)}
		</div>
	);
}

interface DependencyRowViewProps {
	dependency: DependencyRow;
}

/**
 * One row for either direction of the dependency graph -- same treatment as
 * a sidebar arrow row (icon, name) plus the same compact status badge the
 * hero uses, so an entry the user has locally reads as recognizably "that
 * arrow, in that state" rather than a bare namespace string. Always a real
 * `Link`: core's dependency endpoints only ever report installed refs, so
 * the target always resolves, whether or not it happens to be in the
 * reactive catalog too.
 */
function DependencyRowView({ dependency }: DependencyRowViewProps): JSX.Element {
	const { t } = useTranslation();
	const status = computeStatus({ state: dependency.state, user_installed: dependency.userInstalled });
	const StatusIcon = STATUS_ICONS[status.iconKind];

	return (
		<Link
			className="-mx-1 flex items-center gap-2.5 rounded-md px-1 py-1.5 hover:bg-muted"
			params={{ _splat: dependency.namespace }}
			to="/arrow/$"
		>
			<span className="shrink-0" style={{ '--icon': '20px' } as React.CSSProperties}>
				<ArrowIcon icon={dependency.icon} name={dependency.name} namespace={dependency.namespace} />
			</span>
			<span className="min-w-0 flex-1 truncate text-sm">{dependency.name}</span>
			<Badge className="shrink-0 gap-1" size="sm" variant={STATUS_BADGE_VARIANT[status.iconKind]}>
				<StatusIcon aria-hidden="true" className="size-3" />
				{t(status.labelKey)}
			</Badge>
		</Link>
	);
}

/**
 * The "Details" panel: hardware requirements, declared network ports,
 * maintainers/credits, the dependency graph, and the source link. Every
 * section is optional -- omitted entirely (header included) rather than
 * shown empty, since `ArrowDetail` legitimately has none of these for some
 * arrows.
 */
export function MetaPanel({
	requirement,
	netbridge,
	maintainers,
	credits,
	url,
	dependsOn = [],
	requiredBy = [],
}: MetaPanelProps): JSX.Element {
	const { t } = useTranslation();

	return (
		<Frame>
			<FrameHeader>
				<FrameTitle>{t('arrow.details.title')}</FrameTitle>
			</FrameHeader>
			<FramePanel>
				<div className="flex flex-col divide-y divide-border">
					{requirement && (
						<Section title={t('arrow.details.requirements')}>
							<RequirementRow
								icon={CpuIcon}
								label={t('arrow.details.requirements.cpu')}
								value={t('arrow.details.requirements.cpu.value', { count: requirement.cpu_cores })}
							/>
							<RequirementRow
								icon={MemoryStickIcon}
								label={t('arrow.details.requirements.memory')}
								value={t('arrow.details.requirements.memory.value', { count: requirement.memory_gb })}
							/>
							<RequirementRow
								icon={HardDriveIcon}
								label={t('arrow.details.requirements.disk')}
								value={t('arrow.details.requirements.disk.value', { count: requirement.disk_gb })}
							/>
						</Section>
					)}

					{dependsOn.length > 0 && (
						<Section title={t('arrow.details.dependencies')}>
							{dependsOn.map((dependency) => (
								<DependencyRowView dependency={dependency} key={dependency.namespace} />
							))}
						</Section>
					)}

					{netbridge.length > 0 && (
						<Section title={t('arrow.details.network')}>
							{netbridge.map((port) => (
								<PortRow
									key={port.name}
									port={port}
									requiredLabel={t('arrow.details.network.required')}
								/>
							))}
						</Section>
					)}

					{maintainers.length > 0 && (
						<Section title={t('arrow.details.maintainers')}>
							{maintainers.map((credit) => (
								<CreditRow credit={credit} key={creditKey(credit)} />
							))}
						</Section>
					)}

					{credits.length > 0 && (
						<Section title={t('arrow.details.credits')}>
							{credits.map((credit) => (
								<CreditRow credit={credit} key={creditKey(credit)} />
							))}
						</Section>
					)}

					{url && (
						<Section title={t('arrow.details.links')}>
							<div className="flex items-center gap-2.5 py-1.5">
								<LinkIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
								<a
									className="min-w-0 flex-1 break-words text-sm underline-offset-4 hover:underline"
									href={url}
									rel="noreferrer"
									target="_blank"
								>
									{url}
								</a>
							</div>
						</Section>
					)}

					{requiredBy.length > 0 && (
						<Section title={t('arrow.details.requiredBy')}>
							{requiredBy.map((dependency) => (
								<DependencyRowView dependency={dependency} key={dependency.namespace} />
							))}
						</Section>
					)}
				</div>
			</FramePanel>
		</Frame>
	);
}
