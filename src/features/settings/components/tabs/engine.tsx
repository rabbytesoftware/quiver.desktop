import { useEffect, useState } from 'react';

import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Switch } from '@/components/ui/switch';

import { useEngineStore } from '@/features/settings/engine/store';
import { useTranslation } from '@/lib/i18n';

import { Notice, Section, SettingRow } from '../section';

const LEVELS = ['debug', 'info', 'warn', 'error'] as const;

export function EngineSettings() {
	const { t } = useTranslation();
	const view = useEngineStore((s) => s.view);
	const rejected = useEngineStore((s) => s.rejected);
	const error = useEngineStore((s) => s.error);
	const load = useEngineStore((s) => s.load);
	const patch = useEngineStore((s) => s.patch);

	// Bumped to force the uncontrolled port inputs below to remount — and so
	// re-read `defaultValue` from `configured` — whenever a typed value must
	// be thrown away instead of kept on screen: either it was never sent (not
	// a valid integer) or the daemon rejected it. Without this the field would
	// keep showing a value the daemon never actually holds.
	const [startNonce, setStartNonce] = useState(0);
	const [endNonce, setEndNonce] = useState(0);

	useEffect(() => {
		void load();
	}, [load]);

	if (error) {
		return (
			<div className="px-1 py-2">
				<p className="text-xs text-muted-foreground">{error}</p>
				<Button variant="outline" size="sm" className="mt-2" onClick={() => void load()}>
					{t('settings.engine.retry')}
				</Button>
			</div>
		);
	}
	if (!view) return null;

	const { configured, defaults, restart_required: pending, corrected } = view;
	const why = (key: string) => rejected.find((r) => r.key === key)?.message;
	const portProblem = why('netbridge.ephemeral_port_start') ?? why('netbridge.ephemeral_port_end');

	// A blank or non-integer entry is never sent — there is nothing valid to
	// patch — and the field is reverted to what the daemon actually holds. A
	// valid entry is sent, but if the daemon rejects that key the field is
	// reverted the same way, so a refused value can never linger on screen
	// looking accepted.
	async function port(key: 'ephemeral_port_start' | 'ephemeral_port_end', raw: string, revert: () => void) {
		const trimmed = raw.trim();
		const n = Number(trimmed);
		if (trimmed === '' || !Number.isInteger(n)) {
			revert();
			return;
		}
		await patch({ netbridge: { [key]: n } });
		if (useEngineStore.getState().rejected.some((r) => r.key === `netbridge.${key}`)) revert();
	}

	return (
		<div>
			{corrected.length > 0 && (
				<Notice>
					{t('settings.engine.corrected', {
						settings: corrected.map((c) => c.key).join(', '),
					})}
				</Notice>
			)}

			{pending.length > 0 && <Notice>{t('settings.engine.restart')}</Notice>}

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
						    on the daemon's value plus a nonce: without the key a reset (or
						    a revert) would not force the field to re-read `defaultValue`,
						    and a stale number would stay on screen. */}
						<Input
							key={`start-${configured.netbridge.ephemeral_port_start}-${startNonce}`}
							type="number"
							defaultValue={configured.netbridge.ephemeral_port_start}
							onBlur={(e) =>
								void port('ephemeral_port_start', e.target.value, () => setStartNonce((n) => n + 1))
							}
							aria-label={t('settings.engine.ports.lowest')}
							className="h-7 w-[86px]"
						/>
						<span className="text-muted-foreground">–</span>
						<Input
							key={`end-${configured.netbridge.ephemeral_port_end}-${endNonce}`}
							type="number"
							defaultValue={configured.netbridge.ephemeral_port_end}
							onBlur={(e) =>
								void port('ephemeral_port_end', e.target.value, () => setEndNonce((n) => n + 1))
							}
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
