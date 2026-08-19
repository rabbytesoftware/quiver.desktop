import { type JSX, type ReactNode, useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import {
	Dialog,
	DialogClose,
	DialogContent,
	DialogDescription,
	DialogFooter,
	DialogHeader,
	DialogPanel,
	DialogTitle,
} from '@/components/ui/dialog';

import type { DiscoverySummary } from '@/domain/search';
import { cn } from '@/lib/cn';
import type { SearchJob } from '@/lib/core-store/store/search';
import { useTranslation } from '@/lib/i18n';
import { apiFetch } from '@/lib/transport/api';

interface SearchInspectorProps {
	open: boolean;
	onOpenChange: (open: boolean) => void;
	query: string;
	job: SearchJob | null;
	summary: DiscoverySummary | null;
}

interface ConfigResponse {
	running: { search: Record<string, unknown> };
}

const CAPTION = 'text-xs font-medium text-muted-foreground uppercase tracking-wide';
const SOURCE = cn(CAPTION, 'normal-case');

/** Label left, value right -- mono for an identifier or a number, so it doesn't read as prose. */
function InspectorField({ label, value, mono = false }: { label: string; value: ReactNode; mono?: boolean }) {
	return (
		<div className="flex items-center justify-between gap-3">
			<span className="text-muted-foreground">{label}</span>
			<span className={cn('truncate text-right', mono && 'font-mono tabular-nums')}>{value}</span>
		</div>
	);
}

export function SearchInspector({ open, onOpenChange, query, job, summary }: SearchInspectorProps): JSX.Element {
	const { t, formatDateTime } = useTranslation();
	const [settings, setSettings] = useState<Record<string, unknown> | null>(null);

	// Fetched once per open, not on every render -- there is nowhere else in
	// the pass that reads config, and the dialog is the only consumer.
	useEffect(() => {
		if (!open) {
			// Otherwise the next open renders this fetch's config until its own
			// resolves.
			setSettings(null);
			return;
		}
		let cancelled = false;

		apiFetch<ConfigResponse>('/v0/config')
			.then((config) => {
				if (!cancelled) setSettings(config.running.search);
			})
			.catch(() => {
				if (!cancelled) setSettings(null);
			});

		return () => {
			cancelled = true;
		};
	}, [open]);

	const found = summary?.found ?? 0;
	const verified = summary?.verified ?? 0;
	const skipped = summary?.skipped ?? 0;

	return (
		<Dialog onOpenChange={onOpenChange} open={open}>
			<DialogContent>
				<DialogHeader>
					<DialogTitle>{t('search.inspector.title')}</DialogTitle>
					<DialogDescription>{t('search.inspector.description')}</DialogDescription>
				</DialogHeader>

				<DialogPanel className="flex flex-col gap-5 text-sm">
					<section className="flex flex-col gap-1.5">
						<h3 className={CAPTION}>{t('search.inspector.pass')}</h3>
						<div className="flex flex-col gap-1">
							<InspectorField label={t('search.inspector.query')} value={query} />
							{job && <InspectorField label={t('search.inspector.job')} mono value={job.id} />}
							<InspectorField
								label={t('search.inspector.counts')}
								mono
								value={`${found} / ${verified} / ${skipped}`}
							/>
							{job && (
								<InspectorField
									label={t('search.inspector.expires')}
									value={formatDateTime(job.expires_at)}
								/>
							)}
						</div>
					</section>

					<section className="flex flex-col gap-1.5">
						<div className="flex items-center justify-between gap-3">
							<h3 className={CAPTION}>{t('search.inspector.hosts')}</h3>
							{summary && (
								<span className={SOURCE}>
									{t('search.inspector.asked', { count: summary.providers.length })}
								</span>
							)}
						</div>
						{summary ? (
							<ul className="flex flex-col gap-1.5">
								{summary.providers.map((provider) => (
									<li
										className="flex items-center justify-between gap-3"
										data-slot="inspector-host"
										key={provider.host}
									>
										<span className="flex items-center gap-2">
											<span
												aria-hidden
												className={cn(
													'size-2 shrink-0 rounded-full',
													provider.ok ? 'bg-foreground' : 'ring-1 ring-foreground/40'
												)}
											/>
											{provider.host}
										</span>
										<span className="text-muted-foreground">
											{provider.ok
												? t('search.results.count', { count: provider.returned })
												: `${provider.reason ?? ''}${
														provider.retry_after !== null
															? ` · ${t('search.results.retry', { seconds: provider.retry_after })}`
															: ''
													}`}
										</span>
									</li>
								))}
							</ul>
						) : (
							<p className="text-muted-foreground">{t('search.inspector.running')}</p>
						)}
					</section>

					<section className="flex flex-col gap-1.5">
						<div className="flex items-center justify-between gap-3">
							<h3 className={CAPTION}>{t('search.inspector.settings')}</h3>
							<span className={SOURCE}>running.search</span>
						</div>
						{settings && (
							<ul className="flex flex-col gap-1">
								{Object.entries(settings).map(([key, value]) => (
									<li className="flex items-center justify-between gap-3" key={key}>
										<span className="text-muted-foreground">{key}</span>
										<span>{String(value)}</span>
									</li>
								))}
							</ul>
						)}
					</section>
				</DialogPanel>

				<DialogFooter>
					<DialogClose render={<Button variant="outline" />}>{t('search.inspector.close')}</DialogClose>
				</DialogFooter>
			</DialogContent>
		</Dialog>
	);
}
