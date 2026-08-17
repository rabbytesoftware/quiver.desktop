import { useEffect } from 'react';

import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';

import { useEngineStore } from '@/features/settings/engine/store';
import { useTranslation } from '@/lib/i18n';

import { Section, SettingRow } from '../section';

const LEVELS = ['debug', 'info', 'warn', 'error'] as const;

export function EngineSettings() {
	const { t } = useTranslation();
	const view = useEngineStore((s) => s.view);
	const rejected = useEngineStore((s) => s.rejected);
	const error = useEngineStore((s) => s.error);
	const load = useEngineStore((s) => s.load);
	const patch = useEngineStore((s) => s.patch);

	useEffect(() => {
		void load();
	}, [load]);

	if (error) return <p className="px-1 text-xs text-muted-foreground">{error}</p>;
	if (!view) return null;

	const { configured, defaults, restart_required: pending, corrected } = view;
	const why = (key: string) => rejected.find((r) => r.key === key)?.message;
	const portProblem = why('netbridge.ephemeral_port_start') ?? why('netbridge.ephemeral_port_end');

	function port(key: 'ephemeral_port_start' | 'ephemeral_port_end', value: string) {
		const n = Number(value);
		if (!Number.isInteger(n)) return;
		void patch({ netbridge: { [key]: n } });
	}

	return (
		<div>
			{corrected.length > 0 && (
				<p className="mx-1 mb-3 rounded-md border border-border bg-muted/45 px-2.5 py-2 text-xs leading-relaxed text-muted-foreground">
					{t('settings.engine.corrected', {
						settings: corrected.map((c) => c.key).join(', '),
					})}
				</p>
			)}

			{pending.length > 0 && (
				<p className="mx-1 mb-3 rounded-md border border-border bg-muted/45 px-2.5 py-2 text-xs leading-relaxed text-muted-foreground">
					{t('settings.engine.restart')}
				</p>
			)}

			{/* Wrapped so the notices above never disturb the `:first-child` CSS
			    `Section` relies on to hide a panel's leading heading — without
			    this wrapper, a rendered notice would make "Ports" the second
			    element under the root and its heading would reappear. */}
			<div>
				<Section title={t('settings.engine.ports.title')}>
					<SettingRow
						label={t('settings.engine.ports.label')}
						description={portProblem ?? t('settings.engine.ports.description')}
						onReset={() =>
							void patch({ netbridge: { ephemeral_port_start: null, ephemeral_port_end: null } })
						}
						canReset={
							configured.netbridge.ephemeral_port_start !== defaults.netbridge.ephemeral_port_start ||
							configured.netbridge.ephemeral_port_end !== defaults.netbridge.ephemeral_port_end
						}
					>
						{/* Uncontrolled so typing is not fought by a round trip, but keyed
						    on the daemon's value: without the key a reset would persist
						    and the stale number would stay on screen. */}
						<Input
							key={`start-${configured.netbridge.ephemeral_port_start}`}
							type="number"
							defaultValue={configured.netbridge.ephemeral_port_start}
							onBlur={(e) => port('ephemeral_port_start', e.target.value)}
							aria-label={t('settings.engine.ports.lowest')}
							className="h-7 w-[86px]"
						/>
						<span className="text-muted-foreground">–</span>
						<Input
							key={`end-${configured.netbridge.ephemeral_port_end}`}
							type="number"
							defaultValue={configured.netbridge.ephemeral_port_end}
							onBlur={(e) => port('ephemeral_port_end', e.target.value)}
							aria-label={t('settings.engine.ports.highest')}
							className="h-7 w-[86px]"
						/>
					</SettingRow>
				</Section>

				<Section title={t('settings.engine.logs.title')}>
					<SettingRow
						label={t('settings.engine.logs.disk')}
						description={t('settings.engine.logs.diskDescription')}
						onReset={() => void patch({ logger: { enabled: null } })}
						canReset={configured.logger.enabled !== defaults.logger.enabled}
					>
						<Switch
							checked={configured.logger.enabled}
							onCheckedChange={(next) => void patch({ logger: { enabled: next } })}
							aria-label={t('settings.engine.logs.disk')}
						/>
					</SettingRow>

					<SettingRow
						label={t('settings.engine.logs.level')}
						description={why('logger.level') ?? t('settings.engine.logs.levelDescription')}
						onReset={() => void patch({ logger: { level: null } })}
						canReset={configured.logger.level !== defaults.logger.level}
					>
						<Select
							items={LEVELS.map((l) => ({ value: l, label: t(`settings.engine.logs.level.${l}`) }))}
							value={configured.logger.level}
							onValueChange={(next) => void patch({ logger: { level: next } })}
						>
							<SelectTrigger className="w-[112px]" aria-label={t('settings.engine.logs.level')}>
								<SelectValue />
							</SelectTrigger>
							<SelectContent>
								{LEVELS.map((l) => (
									<SelectItem key={l} value={l}>
										{t(`settings.engine.logs.level.${l}`)}
									</SelectItem>
								))}
							</SelectContent>
						</Select>
					</SettingRow>
				</Section>
			</div>
		</div>
	);
}
